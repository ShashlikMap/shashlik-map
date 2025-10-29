use error_stack::{FutureExt, Report, ResultExt};
use old_tiles_gen::source::TileSource;
use old_tiles_gen::source::tiles_sqlite_store::TilesSQLiteStore;
use poem::error::ResponseError;
use poem::http::StatusCode;
use poem::{
    EndpointExt, Result, Route, Server, get, handler,
    listener::TcpListener,
    middleware::AddData,
    web::{Data, Path},
};
use serde::Deserialize;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;
use thiserror::Error;
use tokio::task::spawn_blocking;

#[derive(Error, Debug, Clone)]
enum TileServerError {
    #[error("Internal")]
    Internal,
}

trait DetachReport<T, E> {
    fn detach_report(self) -> Result<T, ReportResponseError<E>>;
}

impl<T, E> DetachReport<T, E> for Result<T, Report<E>> {
    fn detach_report(self) -> Result<T, ReportResponseError<E>> {
        self.map_err(|report| ReportResponseError(report))
    }
}

#[derive(Error, Debug)]
struct ReportResponseError<E>(Report<E>);

impl Display for ReportResponseError<TileServerError> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self.0))
    }
}

impl ResponseError for ReportResponseError<TileServerError> {
    fn status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

#[derive(Deserialize)]
struct TileParam {
    x: i32,
    y: i32,
    z: i32,
}

#[derive(Clone)]
struct AppState {
    tile_source: Arc<dyn TileSource>,
}

#[handler]
async fn get_state(
    Path(TileParam { x, y, z }): Path<TileParam>,
    state: Data<&Arc<AppState>>,
) -> Result<Vec<u8>> {
    println!("getting tile {}/{}/{}", x, y, z);
    let state = state.clone();
    let db_res = spawn_blocking(move || {
        state
            .tile_source
            .fetch(x, y, z)
            .change_context(TileServerError::Internal)
            .detach_report()
    })
    .change_context(TileServerError::Internal)
    .await
    .detach_report()??;
    Ok(db_res)
}

#[tokio::main]
async fn main() -> Result<(), Report<TileServerError>> {
    if std::env::var_os("RUST_LOG").is_none() {
        unsafe {
            std::env::set_var("RUST_LOG", "poem=debug");
        }
    }
    tracing_subscriber::fmt::init();

    let state = Arc::new(AppState {
        tile_source: Arc::new(TilesSQLiteStore::new_default_db()),
    });

    let app = Route::new()
        .at("/tile/:x/:y/:z", get(get_state))
        .with(AddData::new(state));

    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .name("add-data")
        .run(app)
        .await
        .change_context(TileServerError::Internal)
}
