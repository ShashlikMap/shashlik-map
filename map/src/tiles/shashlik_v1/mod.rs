mod shashlik_v1_parser;

use crate::tiles::shashlik_v1::shashlik_v1_parser::ShashlikV1Parser;
use crate::tiles::tiles_provider::{MercatorConverter, MercatorProvider, TilesProviderStore};
use osm::map::{MapGeomObject, MapGeometry};
use osm::tiles::TileKey;
use tiles::Tile;
use tiles::decode::DecodedTile;
use tiles::reader::{FileRangeReader, PmTilesReader, TileSource};
use tiles::view::TilePayload;
use tokio::runtime::{Handle, Runtime};

pub struct ShashlikV1TileStore {
    shashlik_v1_parser: ShashlikV1Parser,
    tokio_rt: Runtime,
    tokio_handle: Handle,
    pm_tiles_reader: PmTilesReader<FileRangeReader>,
}

impl ShashlikV1TileStore {
    pub fn new() -> Self {
        let tokio_rt = Runtime::new().unwrap();
        let tokio_handle = tokio_rt.handle().clone();

        let pm_tiles_reader = tokio_handle.block_on(async move {
            // TODO So far, just some hardcoded path
            let reader = FileRangeReader::open("../../Downloads/japan.pmtiles")
                .await
                .unwrap();
            let pm_reader = PmTilesReader::open(reader).await.unwrap();
            pm_reader
        });

        Self {
            shashlik_v1_parser: ShashlikV1Parser::new(),
            tokio_rt,
            tokio_handle,
            pm_tiles_reader,
        }
    }
}

impl MercatorProvider for ShashlikV1TileStore {}
impl MercatorConverter for ShashlikV1TileStore {}

impl TilesProviderStore for ShashlikV1TileStore {
    fn load(&self, tile_key: &TileKey) -> Vec<(MapGeomObject, MapGeometry<f32>)> {
        let tile_data = self.tokio_handle.block_on(async move {
            let tile_data = self
                .pm_tiles_reader
                .tile(Tile {
                    x: tile_key.tile_x as u32,
                    y: tile_key.tile_y as u32,
                    z: tile_key.zoom_level as u8,
                })
                .await
                .unwrap();
            tile_data
        });
        if let Some(tile_data) = tile_data {
            let decoded_tile = DecodedTile::from_tile_bytes(tile_data);
            let data = self
                .shashlik_v1_parser
                .read_decoded_tile(decoded_tile, tile_key);
            return data;
        }

        vec![]
    }
}
