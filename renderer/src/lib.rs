extern crate core;

use crate::depth_texture::DepthTexture;
use crate::fps::FpsCounter;
use crate::geometry_data::TextData;
use crate::mesh_layers::BaseMeshLayer;
use crate::messages::RendererMessage;
use crate::modifier::render_modifier::SpatialData;
use crate::msaa_texture::MultisampledTexture;
use crate::styles::style_store::StyleStore;
use canvas_api::CanvasApi;
use cgmath::{vec2, vec3, Matrix4, Vector2, Vector3};
use geo_types::Coord;
use global_context::GlobalContext;
use mesh_layers::layers::Layers;
use messages::RendererApiMsg;
use renderer_api::RendererApi;
use rustybuzz::ttf_parser;
use std::collections::HashMap;
use std::iter;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread::spawn;
use tokio::sync::broadcast;
use wgpu::SurfaceError;
use wgpu_canvas::wgpu_canvas::WgpuCanvas;

pub mod canvas_api;
mod collision_handler;
mod consts;
mod depth_texture;
pub mod draw_commands;
mod fps;
pub mod geometry_data;
mod mesh;
pub mod messages;
pub mod modifier;
mod msaa_texture;
pub mod render_group;
pub mod renderer_api;
pub mod styles;
mod svg;
mod text;
pub mod vertex_attrs;
mod view_projection;

pub mod mesh_layers;
pub mod pipelines;
mod utils;
mod global_context;
mod collider;

pub trait Renderer {
    fn resize(&mut self, width: u32, height: u32);
    fn update(&mut self, view_proj_matrix: Matrix4<f64>, cs_offset: Vector3<f64>);
    fn render(&mut self) -> Result<(), SurfaceError>;
}

pub struct ShashlikRenderer {
    layers: Layers,
    depth_texture: DepthTexture,
    msaa_texture: MultisampledTexture,
    renderer_rx: Receiver<RendererMessage>,
    pub api: Arc<RendererApi>,
    fps_counter: FpsCounter<100>,
    global_context: GlobalContext,
}

impl ShashlikRenderer {
    pub async fn new(
        feature_tags: &[String],
        canvas: Box<dyn WgpuCanvas>,
        font: &'static ttf_parser::Face<'static>,
    ) -> anyhow::Result<ShashlikRenderer> {
        let style_store = StyleStore::new();

        let mut global_context = GlobalContext::new(canvas, &style_store);

        let depth_texture = DepthTexture::new(&global_context);
        let msaa_texture = MultisampledTexture::new(&global_context);
        
        let mut layers = Layers::new(feature_tags, &mut global_context, font);

        layers.text_layer.add("fps_info".to_string(), vec![TextData {
            id: 0,
            text: "FPS 0".to_string(),
            size: 40.0,
            alpha: 1.0,
            positions: vec![vec3(100.0, 120.0, 0.0)],
            screen_offset: vec2(0.0, 0.0),
            screen_space: true,
            glyph_buffer: None,
        }], SpatialData::new());

        let (renderer_api_tx, renderer_api_rx) = channel();

        let (renderer_tx, renderer_rx) = channel();
        Self::run_background(style_store, renderer_tx.clone(), renderer_api_rx);

        let api = Arc::new(RendererApi::new(renderer_api_tx));

        layers.prepare(&mut global_context);

        Ok(Self {
            layers,
            depth_texture,
            msaa_texture,
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

            self.depth_texture = DepthTexture::new(&self.global_context);
            self.msaa_texture = MultisampledTexture::new(&self.global_context);
        }
    }

    fn update(&mut self, view_proj_matrix: Matrix4<f64>, cs_offset: Vector3<f64>) {
        self.global_context.update(view_proj_matrix, cs_offset);

        if let Ok(message) = self.renderer_rx.try_recv() {
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

    fn render(&mut self) -> Result<(), SurfaceError> {
        self.global_context.canvas.on_pre_render();
        // // We can't render unless the surface is configured
        // if !self.is_surface_configured {
        //     return Ok(());
        // }

        let output = self.global_context.canvas.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            self.global_context
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.msaa_texture.view,
                    resolve_target: Some(&view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.741,
                            b: 0.961,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            let fps = format!("FPS {}", self.fps_counter.update() as i32);
            self.layers.text_layer.run_mut_action_with_key("fps_info", move |item| {
                item.update_text(fps.as_str(), 1.0);
            });

            self.layers
                .render(&mut render_pass, &mut self.global_context);
        }

        self.global_context
            .queue()
            .submit(iter::once(encoder.finish()));
        output.present();

        self.global_context.canvas.on_post_render();

        Ok(())
    }
}

impl Renderer for ShashlikRenderer {
    fn resize(&mut self, width: u32, height: u32) {
        self.resize(width, height);
    }

    fn update(&mut self, view_proj_matrix: Matrix4<f64>, cs_offset: Vector3<f64>) {
        self.update(view_proj_matrix, cs_offset);
    }

    fn render(&mut self) -> Result<(), SurfaceError> {
        self.render()
    }
}
