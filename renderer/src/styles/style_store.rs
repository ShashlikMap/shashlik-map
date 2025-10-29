use crate::consts::STYLE_SHADER_PARAMS_COUNT;
use crate::styles::render_style::RenderStyle;
use crate::styles::style_id::StyleId;
use indexmap::IndexMap;
use log::error;
use tokio::sync::broadcast::{Receiver, Sender};

#[derive(Clone)]
pub struct StyleStore {
    uniform_tx: Sender<Vec<[f32; STYLE_SHADER_PARAMS_COUNT]>>,
    style_map: IndexMap<StyleId, RenderStyle>,
}

impl StyleStore {
    const STUB_STYLE_ID: StyleId = StyleId("stub");
    pub fn new() -> StyleStore {
        let (uniform_tx, _) = tokio::sync::broadcast::channel(1);
        let mut store = StyleStore {
            uniform_tx,
            style_map: IndexMap::new(),
        };
        store.register_styles(vec![(Self::STUB_STYLE_ID, RenderStyle::default())]);
        store
    }

    pub fn register_styles(&mut self, styles: Vec<(StyleId, RenderStyle)>) {
        styles.into_iter().for_each(|(style_id, style)| {
            self.style_map.insert(style_id, style);
        });
        self.generate_uniforms_and_send();
    }

    fn styles(&self) -> Vec<&RenderStyle> {
        self.style_map.values().collect()
    }

    pub fn subscribe(&self) -> Receiver<Vec<[f32; STYLE_SHADER_PARAMS_COUNT]>> {
        let receiver = self.uniform_tx.subscribe();
        self.generate_uniforms_and_send();
        receiver
    }

    fn generate_uniforms_and_send(&self) {
        let styles = self
            .styles()
            .iter()
            .map(|it| it.params())
            .collect::<Vec<_>>();
        if self.uniform_tx.receiver_count() > 0 {
            self.uniform_tx.send(styles).unwrap();
        } else {
            error!("No uniform_tx in style_store");
        }
    }

    pub fn get_index(&mut self, style_id: &StyleId) -> usize {
        self.style_map
            .entry(style_id.clone())
            .or_insert(RenderStyle::default());
        let (index, _, _) = self.style_map.get_full(style_id).unwrap();
        index
    }

    pub fn update_style<F: FnOnce(&mut RenderStyle)>(&mut self, style_id: &StyleId, updater: F) {
        let style = self.style_map.entry(style_id.clone()).or_insert(RenderStyle::default());
        updater(style);
        self.generate_uniforms_and_send();
    }
}
