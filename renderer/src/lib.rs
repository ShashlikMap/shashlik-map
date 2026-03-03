extern crate core;

use crate::fps::FpsCounter;
use crate::geometry_data::TextData;
use crate::mesh_layers::BaseMeshLayer;
use crate::messages::RendererMessage;
use crate::modifier::render_modifier::SpatialData;
use crate::pass_nodes::PassNode;
use crate::pass_nodes::main_pass_node::MainPassNode;
use crate::pass_nodes::render_to_texture_pass_node::RenderToTexturePassNode;
use crate::styles::style_store::StyleStore;
use canvas_api::CanvasApi;
use cgmath::{Matrix4, Vector2, Vector3, vec2, vec3};
use geo_types::Coord;
use global_context::GlobalContext;
use mesh_layers::layers::Layers;
use messages::RendererApiMsg;
use renderer_api::RendererApi;
use rustybuzz::ttf_parser;
use std::collections::HashMap;
use std::iter;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::spawn;
use tokio::sync::broadcast;
use wgpu::Texture;
use wgpu_canvas::wgpu_canvas::WgpuCanvas;
use crate::mesh_layers::feature_layers::FeatureLayerTag;
use crate::pass_nodes::prepass_node::PrepassNode;

pub mod canvas_api;
mod collision_handler;
mod consts;
pub mod draw_commands;
mod fps;
pub mod geometry_data;
pub mod mesh;
pub mod messages;
pub mod modifier;
pub mod render_group;
pub mod renderer_api;
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

pub trait Renderer {
    fn resize(&mut self, width: u32, height: u32);
    fn update(&mut self, view_proj_matrix: Matrix4<f64>, cs_offset: Vector3<f64>, scale: f32);
    fn render(&mut self) -> Option<Texture>;
}

pub struct ShashlikRenderer {
    layers: Layers,
    pass_nodes: Vec<Box<dyn PassNode>>,
    renderer_rx: Receiver<RendererMessage>,
    pub api: Arc<RendererApi>,
    fps_counter: FpsCounter<100>,
    global_context: GlobalContext,
}

impl ShashlikRenderer {
    pub async fn new(
        feature_tags: Vec<FeatureLayerTag>,
        canvas: Box<dyn WgpuCanvas>,
        font: &'static ttf_parser::Face<'static>,
    ) -> anyhow::Result<ShashlikRenderer> {
        let style_store = StyleStore::new();

        let mut global_context = GlobalContext::new(canvas, &style_store);

        let mut layers = Layers::new(feature_tags, &mut global_context, font);

        layers.text_layer.add(
            "fps_info".to_string(),
            vec![TextData {
                id: 0,
                text: "FPS 0".to_string(),
                size: 40.0,
                alpha: 1.0,
                positions: vec![vec3(100.0, 120.0, 0.0)],
                screen_offset: vec2(0.0, 0.0),
                screen_space: true,
                glyph_buffer: None,
            }],
            SpatialData::new(),
        );

        let (renderer_api_tx, renderer_api_rx) = channel();

        let (renderer_tx, renderer_rx) = channel();
        Self::run_background(style_store, renderer_tx.clone(), renderer_api_rx);

        let api = Arc::new(RendererApi::new(renderer_api_tx));

        layers.prepare(&mut global_context);

        Ok(Self {
            layers,
            pass_nodes: vec![],
            renderer_rx,
            api,
            fps_counter: FpsCounter::new(),
            global_context,
        })
    }

    fn run_background(
        style_store: StyleStore,
        renderer_tx: Sender<RendererMessage>,
        receiver_api_rx: Receiver<RendererApiMsg>,
    ) {
        spawn(move || {
            let mut canvas_api = CanvasApi::new(style_store);
            let mut spatial_data_map = HashMap::new();
            loop {
                if let Some(api_msg) = receiver_api_rx.recv().ok() {
                    match api_msg {
                        RendererApiMsg::RenderGroup((key, spatial_data, mut rg)) => {
                            let (spatial_tx, _) = broadcast::channel(1);
                            spatial_data_map
                                .insert(key.clone(), (spatial_data.clone(), spatial_tx.clone()));

                            canvas_api.start_commands();
                            rg.content(&mut canvas_api);
                            let commands = canvas_api.flush_commands(key, spatial_data, spatial_tx);

                            renderer_tx.send(RendererMessage::Draw(commands)).unwrap();
                        }
                        RendererApiMsg::UpdateStyle((style, block)) => {
                            canvas_api.update_style(&style, block);
                        }
                        RendererApiMsg::UpdateSpatialData((key, spatial_data_cb)) => {
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

    pub fn clip_to_world(&self, coord: &Coord<f64>) -> Option<Vector2<f64>> {
        self.global_context.view_projection.clip_to_world(coord)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.global_context.resize(width, height);

            self.config_pass_nodes();
        }
    }

    fn config_pass_nodes(&mut self) {
        let pre_pass_node = PrepassNode::new();

        let rt_node = RenderToTexturePassNode::new(&mut self.global_context);
        self.layers
            .ortho_mesh_layer
            .set_texture(&rt_node.rt_texture_view, &self.global_context);

        let main_node = MainPassNode::new(&mut self.global_context);
        self.pass_nodes = vec![Box::new(pre_pass_node), Box::new(rt_node), Box::new(main_node)];
    }

    fn update(&mut self, view_proj_matrix: Matrix4<f64>, cs_offset: Vector3<f64>, scale: f32) {
        self.global_context.update(view_proj_matrix, cs_offset, scale);

        // read all messages between renders
        for message in self.renderer_rx.try_iter() {
            match message {
                RendererMessage::Draw(mut draw_commands) => {
                    draw_commands.execute(&mut self.global_context, &mut self.layers);
                }
                RendererMessage::ClearGroups(keys) => {
                    keys.into_iter().for_each(|key| {
                        self.layers.clear_by_key(&*key);
                    });
                }
            }
        }

        self.layers.update(&mut self.global_context);
    }

    fn render(&mut self) -> Option<Texture> {
        // // We can't render unless the surface is configured
        // if !self.is_surface_configured {
        //     return Ok(());
        // }

        let output_view = self.global_context.canvas.create_texture_view();

        let mut encoder =
            self.global_context
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        let fps = format!("FPS {}", self.fps_counter.update() as i32);
        self.layers
            .text_layer
            .run_mut_action_with_key("fps_info", move |item| {
                item.update_text(fps.as_str(), 1.0);
            });

        self.pass_nodes.iter_mut().for_each(|node| {
            node.compute(&mut encoder,
                         &mut self.layers,
                         &mut self.global_context);

            node.render(
                &mut encoder,
                &output_view,
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

impl Renderer for ShashlikRenderer {
    fn resize(&mut self, width: u32, height: u32) {
        self.resize(width, height);
    }

    fn update(&mut self, view_proj_matrix: Matrix4<f64>, cs_offset: Vector3<f64>, scale: f32) {
        self.update(view_proj_matrix, cs_offset, scale);
    }

    fn render(&mut self) -> Option<Texture> {
        self.render()
    }
}
