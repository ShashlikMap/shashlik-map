extern crate core;

use crate::collision_handler::CollisionHandler;
use crate::depth_texture::DepthTexture;
use crate::fps::FpsCounter;
use crate::geometry_data::TextData;
use crate::layers::Layers;
use crate::mesh_layers::BaseMeshLayer;
use crate::messages::RendererMessage;
use crate::msaa_texture::MultisampledTexture;
use mesh_layers::feature_layers::FeatureLayers;
use crate::styles::style_store::StyleStore;
use crate::view_projection::ViewProjection;
use canvas_api::CanvasApi;
use cgmath::{Matrix4, Vector2, Vector3, vec2, vec3};
use geo_types::Coord;
use messages::RendererApiMsg;
use renderer_api::RendererApi;
use rustybuzz::ttf_parser;
use std::collections::HashMap;
use std::iter;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::spawn;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::TryRecvError;
use wgpu::{Device, SurfaceError};
use wgpu_canvas::wgpu_canvas::WgpuCanvas;

pub mod canvas_api;
mod collision_handler;
mod consts;
mod depth_texture;
pub mod draw_commands;
mod fps;
pub mod geometry_data;
mod layers;
mod mesh;
pub mod messages;
pub mod modifier;
mod msaa_texture;
pub mod nodes;
pub mod render_group;
pub mod renderer_api;
pub mod styles;
mod svg;
mod text;
pub mod vertex_attrs;
mod view_projection;

pub mod mesh_layers;
pub mod pipelines;

pub const SHADER_STYLE_GROUP_INDEX: u32 = 1;

pub trait Renderer {
    fn resize(&mut self, width: u32, height: u32);
    fn update(&mut self, view_proj_matrix: Matrix4<f64>, cs_offset: Vector3<f64>);
    fn render(&mut self) -> Result<(), SurfaceError>;
}

pub trait ReceiverExt<T: Clone> {
    fn no_lagged(&mut self) -> Result<T, TryRecvError>;
}

impl<T: Clone> ReceiverExt<T> for tokio::sync::broadcast::Receiver<T> {
    fn no_lagged(&mut self) -> Result<T, TryRecvError> {
        let result = self.try_recv();
        if let Err(err) = &result {
            match err {
                TryRecvError::Lagged(_) => return self.no_lagged(),
                _ => {}
            }
        }
        result
    }
}

pub struct GlobalContext {
    view_projection: ViewProjection,
    collision_handler: CollisionHandler,
}

impl GlobalContext {
    pub fn new(device: &Device, collision_handler: CollisionHandler) -> Self {
        GlobalContext {
            view_projection: ViewProjection::new(device),
            collision_handler,
        }
    }
}

pub struct ShashlikRenderer {
    layers: Layers,
    depth_texture: DepthTexture,
    msaa_texture: MultisampledTexture,
    canvas: Box<dyn WgpuCanvas>,
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
        let device = canvas.device();
        let config = canvas.config();

        let depth_texture = DepthTexture::new(&device, config.width, config.height);
        let msaa_texture =
            MultisampledTexture::new(device, config.width, config.height, config.format);

        let mut global_context = GlobalContext::new(device, CollisionHandler::new(
            config.width as f32,
            config.height as f32,
        ));
        global_context
            .view_projection
            .resize(config.width, config.height);

        let style_store = StyleStore::new();

        let feature_layers = FeatureLayers::new(feature_tags, &device, &mut global_context, &style_store);

        let mut layers = Layers::new(device, &mut global_context, feature_layers, &style_store, font);

        let (renderer_api_tx, renderer_api_rx) = channel();

        let (renderer_tx, renderer_rx) = channel();
        Self::run_background(style_store, renderer_tx.clone(), renderer_api_rx);

        let api = Arc::new(RendererApi::new(renderer_api_tx));

        layers.prepare(&mut global_context, device, config);

        Ok(Self {
            layers,
            depth_texture,
            msaa_texture,
            canvas,
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
                let api_msg = receiver_api_rx.recv().unwrap();
                match api_msg {
                    // TODO remove layer
                    RendererApiMsg::RenderGroup((key, _layer, spatial_data, mut rg)) => {
                        let (spatial_tx, _) = broadcast::channel(1);
                        spatial_data_map
                            .insert(key.clone(), (spatial_data.clone(), spatial_tx.clone()));

                        canvas_api.begin_shape();
                        rg.content(&mut canvas_api);
                        canvas_api.flush();

                        let commands = canvas_api.draw_commands(key, spatial_data, spatial_tx);
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
        });
    }

    pub fn clip_to_world(&self, coord: &Coord<f64>) -> Option<Vector2<f64>> {
        self.global_context.view_projection.clip_to_world(coord)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.canvas.on_resize();
            let config = self.canvas.config();
            let device = self.canvas.device();

            self.global_context
                .view_projection
                .resize(config.width, config.height);
            self.global_context
                .collision_handler
                .resize(config.width as f32, config.height as f32);

            self.depth_texture = DepthTexture::new(&device, config.width, config.height);
            self.msaa_texture =
                MultisampledTexture::new(device, config.width, config.height, config.format);
        }
    }

    fn update(&mut self, view_proj_matrix: Matrix4<f64>, cs_offset: Vector3<f64>) {
        let device = self.canvas.device();
        let queue = self.canvas.queue();
        let config = self.canvas.config();

        self.global_context
            .view_projection
            .update(queue, config, view_proj_matrix, cs_offset);
        if let Ok(message) = self.renderer_rx.try_recv() {
            match message {
                RendererMessage::Draw(mut draw_commands) => {
                    draw_commands.execute(&device, &mut self.layers);
                }
                RendererMessage::ClearGroups(keys) => {
                    keys.into_iter().for_each(|key| {
                        self.layers.clear(key);
                    });
                }
            }
        }

        self.global_context.collision_handler.clear();
    }

    fn render(&mut self) -> Result<(), SurfaceError> {
        self.canvas.on_pre_render();
        // // We can't render unless the surface is configured
        // if !self.is_surface_configured {
        //     return Ok(());
        // }

        let output = self.canvas.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let device = self.canvas.device();
        let queue = self.canvas.queue();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
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

            // TODO can we do it better?
            self.layers.new_text_layer.text_renderer.insert(
                &mut TextData {
                    id: 0,
                    text: format!("FPS {}", self.fps_counter.update() as i32),
                    size: 40.0,
                    alpha: 1.0,
                    positions: vec![vec3(100.0, 120.0, 0.0)],
                    screen_offset: vec2(0.0, 0.0),
                    screen_space: true,
                    glyph_buffer: None,
                },
                &mut self.global_context.collision_handler,
                &self.global_context.view_projection,
            );
            self.layers
                .render(&mut render_pass, queue, device, &mut self.global_context);
        }

        queue.submit(iter::once(encoder.finish()));
        output.present();

        self.canvas.on_post_render();

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
