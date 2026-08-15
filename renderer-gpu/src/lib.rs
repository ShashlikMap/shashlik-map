extern crate core;

use crate::buffer_pool::BufferPool;
use crate::mesh_layers::layers::SCREEN_TEXT_LAYER;
use crate::messages::RendererMessage;
use crate::pass_nodes::PassNode;
use crate::pass_nodes::main_pass_node::MainPassNode;
use crate::pass_nodes::prepass_node::PrepassNode;
use crate::pass_nodes::render_to_texture_pass_node::RenderToTexturePassNode;
use crate::pass_nodes::shadow_pre_pass::ShadowPrepass;
use crate::styles::style_store::StyleStore;
use crate::wgpu_canvas::WgpuCanvas;
use canvas_api::GpuCanvasApi;
use geo_types::Coord;
use glam::{DVec2, dvec3, vec2};
use global_context::GlobalContext;
use mesh_layers::layers::Layers;
use renderer_common::fps::FpsCounter;
use ::renderer_common::geometry_data::{LineData, TextData};
use renderer_common::r_api_messenger::{CommonRendererApi, RendererApiMsg};
use ::renderer_common::render_modifier::SpatialData;
use ::renderer_common::{PreviewType, Renderer, RendererUpdateData, WorldShapeFeatureLayerTag};
use rustybuzz::ttf_parser::Face;
use std::collections::HashMap;
use std::iter;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::spawn;
use strum::IntoEnumIterator;
use tokio::sync::broadcast;
use wgpu::{Texture, TextureView};
use crate::pass_nodes::g_buffer_pass_node::GBufferPassNode;
use crate::pass_nodes::ssao_pass_node::SsaoPassNode;
use crate::render_config::RenderConfig;
use crate::texture_view_resources::TextureViewKind;

pub mod canvas_api;
pub mod draw_commands;
pub mod mesh;
pub mod messages;
pub mod styles;
mod svg;
mod text;
pub mod vertex_attrs;
mod view_projection;

mod collider;
mod global_context;
pub mod mesh_layers;
mod pass_nodes;
pub mod pipelines;
mod textures;
mod utils;
mod buffer_pool;

pub mod wgpu_canvas;
pub mod render_config;

pub(crate) mod texture_view_resources;
mod mesh_buffers;
pub(crate) mod bind_group_cache;

pub struct GpuRenderer {
    render_config: RenderConfig,
    layers: Layers,
    pass_nodes: Vec<Box<dyn PassNode>>,
    renderer_rx: Receiver<RendererMessage>,
    pub api: Arc<CommonRendererApi<GpuCanvasApi>>,
    fps_counter: FpsCounter<100>,
    global_context: GlobalContext,
    buffer_pool: BufferPool,
    preview_textures: HashMap<PreviewType, TextureView>,
}

impl GpuRenderer {
    pub async fn new(feature_tags: Vec<WorldShapeFeatureLayerTag>,
                     canvas: Box<dyn WgpuCanvas>,
                     font_data: &'static [u8]) -> anyhow::Result<GpuRenderer> {
        Self::new_with_config(RenderConfig::default(), feature_tags, canvas, font_data).await
    }
    pub async fn new_with_config(
        render_config: RenderConfig,
        feature_tags: Vec<WorldShapeFeatureLayerTag>,
        canvas: Box<dyn WgpuCanvas>,
        font_data: &'static [u8],
    ) -> anyhow::Result<GpuRenderer> {
        let style_store = StyleStore::new();
        let mut global_context = GlobalContext::new(canvas, &render_config, &style_store);

        let font = Face::parse(font_data, 0)?;
        let mut layers = Layers::new(feature_tags, &mut global_context, font);
        
        layers.text_feature_layers.get_layer(SCREEN_TEXT_LAYER).unwrap().add(
            "fps_info".to_string(),
            vec![TextData::screen_space_new(0, "FPS 0".to_string(),
                                            vec2(0.0, 0.0), 40.0,
                                            LineData::new(vec![dvec3(100.0, 120.0, 0.0)]))],
            SpatialData::new(),
        );

        let (renderer_api_tx, renderer_api_rx) = channel();

        let (renderer_tx, renderer_rx) = channel();
        Self::run_background(style_store, renderer_tx.clone(), renderer_api_rx);

        let api = Arc::new(CommonRendererApi::new(renderer_api_tx));
        
        Ok(Self {
            render_config,
            layers,
            pass_nodes: vec![],
            renderer_rx,
            api,
            fps_counter: FpsCounter::new(),
            global_context,
            buffer_pool: BufferPool::new(),
            preview_textures: HashMap::new(),
        })
    }

    fn run_background(
        style_store: StyleStore,
        renderer_tx: Sender<RendererMessage>,
        receiver_api_rx: Receiver<RendererApiMsg<GpuCanvasApi>>,
    ) {
        spawn(move || {
            let mut canvas_api = GpuCanvasApi::new(style_store);
            let mut spatial_data_map = HashMap::new();
            loop {
                if let Some(api_msg) = receiver_api_rx.recv().ok() {
                    match api_msg {
                        RendererApiMsg::RenderGroup(key, spatial_data, mut rg) => {
                            let (spatial_tx, _) = broadcast::channel(1);
                            spatial_data_map
                                .insert(key.clone(), (spatial_data.clone(), spatial_tx.clone()));

                            canvas_api.start_commands();
                            rg.content(&mut canvas_api);
                            let commands = canvas_api.flush_commands(key, spatial_data, spatial_tx);

                            renderer_tx.send(RendererMessage::Draw(commands)).unwrap();
                        }
                        RendererApiMsg::UpdateStyle(style, block) => {
                            canvas_api.update_style(&style, block);
                        }
                        RendererApiMsg::UpdateSpatialData(key, spatial_data_cb) => {
                            if let Some((spatial_data, tx)) = spatial_data_map.get_mut(&key) {
                                spatial_data_cb(spatial_data);
                                if tx.receiver_count() > 0 {
                                    tx.send(spatial_data.clone()).unwrap();
                                }
                            }
                        }
                        RendererApiMsg::ClearGroups(keys) => {
                            keys.iter().for_each(|key| {
                                spatial_data_map.remove(key);
                            });
                            renderer_tx
                                .send(RendererMessage::ClearGroups(keys))
                                .unwrap();
                        }
                    }
                }
            }
        });
    }

    pub fn clip_to_world(&self, coord: &Coord<f64>) -> Option<DVec2> {
        self.global_context.view_projection.clip_to_world(coord)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.global_context.resize(width, height);

            self.config_pass_nodes_and_textures();
        }
    }

    pub fn update_config(&mut self, action: impl Fn(&mut RenderConfig)) {
        action(&mut self.render_config);

        self.config_pass_nodes_and_textures();
    }

    fn config_pass_nodes_and_textures(&mut self) {
        let pre_pass_node = PrepassNode::new();
        self.pass_nodes = vec![Box::new(pre_pass_node)];

        if self.render_config.shadow_enabled {
            let shadow_pass_node = ShadowPrepass::new(&self.global_context);
            self.pass_nodes.push(Box::new(shadow_pass_node));

            self.layers
                .shadow_map_layer
                .set_texture(&self.global_context.texture_view_resources.get_or_unwrap(TextureViewKind::ShadowMapDepth), (0.0, 0.0), &self.global_context, &mut self.buffer_pool);
        }

        if self.render_config.ssao_enabled {
            let g_buf_node = GBufferPassNode::new(&mut self.global_context);
            self.pass_nodes.push(Box::new(g_buf_node));

            let ssao_node = SsaoPassNode::new(&mut self.global_context);
            self.pass_nodes.push(Box::new(ssao_node));

            self.layers
                .post_process_layer
                .set_texture(self.global_context.texture_view_resources.get_or_unwrap(TextureViewKind::SSAO),
                             (0.0, 0.0), &self.global_context, &mut self.buffer_pool);
        }

        self.preview_textures.clear();
        if self.render_config.preview_type != PreviewType::None {
            let rt_node = RenderToTexturePassNode::new(&mut self.global_context,
                                                       self.layers.world_shapes_feature_tags.clone());

            let texture_view_resources = &self.global_context.texture_view_resources;
            PreviewType::iter().for_each(|preview_type| {
                if let Some(texture_view) = match preview_type {
                    PreviewType::None => None,
                    PreviewType::Camera => Some(rt_node.rt_texture_view.clone()),
                    PreviewType::SSAO => {
                        texture_view_resources.get(TextureViewKind::SSAO).cloned()
                    },
                    PreviewType::GBufPositions => {
                        texture_view_resources.get(TextureViewKind::GBufPositions).cloned()
                    }
                    PreviewType::GBufNormals => {
                        texture_view_resources.get(TextureViewKind::GBufNormals).cloned()
                    }
                    PreviewType::GBufDepth => {
                        texture_view_resources.get(TextureViewKind::GBufDepth).cloned()
                    }
                    PreviewType::ShadowMap => {
                        texture_view_resources.get(TextureViewKind::ShadowMapDepth).cloned()
                    },
                } {
                    self.preview_textures.insert(preview_type, texture_view);
                }
            });

            self.pass_nodes.push(Box::new(rt_node));
        }

        if let Some(texture_view) = self.preview_textures.get(&self.render_config.preview_type) {
            self.layers
                .preview_mesh_layer
                .set_texture(texture_view, (-100.0, -100.0), &self.global_context, &mut self.buffer_pool);
        }

        let main_node = MainPassNode::new(&mut self.global_context,
                                          &self.layers,
                                          self.layers.world_shapes_feature_tags.clone());
        self.pass_nodes.push(Box::new(main_node));
    }

    pub fn update(&mut self, data: RendererUpdateData) {
        self.global_context.update(&self.render_config, data);

        // read all messages between renders
        for message in self.renderer_rx.try_iter() {
            match message {
                RendererMessage::Draw(mut draw_commands) => {
                    draw_commands.execute(&mut self.global_context, &mut self.layers, &mut self.buffer_pool);
                }
                RendererMessage::ClearGroups(keys) => {
                    keys.into_iter().for_each(|key| {
                        self.layers.clear_by_key(&*key);
                        self.buffer_pool.recycle(key.as_str())
                    });
                }
            }
        }

        self.layers.update(&mut self.global_context);
    }

    pub fn render(&mut self) -> Option<Texture> {
        let mut encoder =
            self.global_context
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        let fps = format!("FPS {}", self.fps_counter.update() as i32);
        self.layers
            .text_feature_layers.get_layer(SCREEN_TEXT_LAYER)?
            .run_mut_action_with_key("fps_info", move |item| {
                item.update_text(fps.as_str(), 1.0);
            });

        self.pass_nodes.iter_mut().for_each(|node| {
            node.run(
                &mut encoder,
                &mut self.layers,
                &mut self.global_context,
            );
        });

        self.global_context
            .queue()
            .submit(iter::once(encoder.finish()));

        self.global_context.canvas.present()
    }
}

impl Renderer for GpuRenderer {
    type RAPI = CommonRendererApi<GpuCanvasApi>;
    type OUTPUT = Texture;
    type INPUT<'a> = ();

    fn screen_size(&self) -> (f32, f32) {
        let config = self.global_context.canvas.config();
        (config.width as f32, config.height as f32)
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.resize(width, height);
    }

    fn update(&mut self, data: RendererUpdateData) {
        self.update(data);
    }

    fn clip_to_world(&self, coord: &Coord) -> Option<DVec2> {
        self.clip_to_world(coord)
    }

    fn render(&mut self, _input: Self::INPUT<'_>) -> Option<Self::OUTPUT> {
        self.render()
    }

    fn api(&self) -> Arc<CommonRendererApi<GpuCanvasApi>> {
        Arc::clone(&self.api)
    }
}
