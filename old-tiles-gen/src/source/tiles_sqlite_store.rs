use error_stack::{Report, ResultExt};
use rusqlite::{named_params, Connection, OpenFlags};
use std::path::Path;
use std::sync::Mutex;
use thiserror::Error;

pub struct TilesSQLiteStore {
    db_conn: Mutex<Connection>,
}
#[derive(Debug, Error)]
pub enum TilesSQLiteStoreError {
    #[error("SqliteError")]
    SqliteError,
    #[error("MissingData")]
    MissingData,
}

impl TilesSQLiteStore {
    const TILE_QUERY: &'static str = "SELECT data FROM tiles WHERE x=:x AND y=:y AND z=:z;";
    pub fn new<P: AsRef<Path>>(path: P) -> TilesSQLiteStore {
        Self {
            db_conn: Mutex::new(Self::create_tiles_db_connection(path)),
        }
    }

    pub fn new_default_db() -> TilesSQLiteStore {
        Self::new("./old-tiles-gen/dbs/tiles.db")
    }

    fn create_tiles_db_connection<P: AsRef<Path>>(path: P) -> Connection {
        Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY
                | OpenFlags::SQLITE_OPEN_NO_MUTEX
                | OpenFlags::SQLITE_OPEN_URI,
        )
        .unwrap()
    }

    pub fn get_tile(
        &self,
        x: i32,
        y: i32,
        z: i32,
    ) -> Result<Vec<u8>, Report<TilesSQLiteStoreError>> {
        let tile_data = self
            .get_tile_internal(x, y, z)
            .change_context(TilesSQLiteStoreError::SqliteError)?;
        tile_data.ok_or(TilesSQLiteStoreError::MissingData.into())
    }

    fn get_tile_internal(&self, x: i32, y: i32, z: i32) -> rusqlite::Result<Option<Vec<u8>>> {
        let conn = self.db_conn.lock().unwrap();
        conn.prepare(Self::TILE_QUERY)?
            .query_and_then(
                named_params! {
                    ":x": x.to_string().as_str(),
                    ":y": y.to_string().as_str(),
                    ":z": z.to_string().as_str()
                },
                |row| row.get::<_, Vec<u8>>(0),
            )?
            .next()
            .map_or(Ok(None), |data| data.map(|data| Some(data)))
    }
}
