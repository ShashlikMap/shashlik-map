extern crate core;

use crate::collision_handler::CollisionHandler;
use crate::depth_texture::DepthTexture;
use crate::layers::Layers;
use crate::messages::RendererMessage;
use crate::msaa_texture::MultisampledTexture;
use crate::nodes::camera_node::CameraNode;
use crate::nodes::mesh_layer::MeshLayer;
use crate::nodes::scene_tree::{RenderContext, SceneTree};
use crate::nodes::shape_layers::ShapeLayers;
use crate::nodes::style_adapter_node::StyleAdapterNode;
use crate::nodes::world::World;
use crate::nodes::SceneNode;
use crate::pipeline_provider::PipeLineProvider;
use crate::styles::style_store::StyleStore;
use crate::text::create_default_text_brush;
use crate::text::text_renderer::TextRenderer;
use crate::vertex_attrs::{InstancePos, ShapeVertex, VertexAttrib, VertexNormal};
use camera::CameraController;
use canvas_api::CanvasApi;
use messages::RendererApiMsg;
use renderer_api::RendererApi;
use std::cell::RefCell;
use std::collections::HashMap;
use std::iter;
use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread::spawn;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::TryRecvError;
use wgpu::{include_wgsl, CompareFunction, DepthStencilState, Face, SurfaceError, TextureFormat};
use wgpu_canvas::wgpu_canvas::WgpuCanvas;
use wgpu_text::glyph_brush::ab_glyph::FontRef;
use wgpu_text::TextBrush;

pub mod camera;
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
pub mod nodes;
mod pipeline_provider;
pub mod render_group;
pub mod renderer_api;
pub mod styles;
mod svg;
mod text;
pub mod vertex_attrs;
mod layers;

pub const SHADER_STYLE_GROUP_INDEX: u32 = 1;

pub trait Renderer {
    fn resize(&mut self, width: u32, height: u32);
    fn update_and_render(&mut self) -> Result<(), SurfaceError>;

    fn update(&mut self);

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
    camera_controller: Rc<RefCell<CameraController>>,
    collision_handler: CollisionHandler,
    text_renderer: TextRenderer,
}

impl GlobalContext {
    pub fn new(
        camera_controller: Rc<RefCell<CameraController>>,
        collision_handler: CollisionHandler,
        text_brush: TextBrush<FontRef<'static>>,
        device: &wgpu::Device,
    ) -> Self {
        GlobalContext {
            camera_controller,
            collision_handler,
            text_renderer: TextRenderer::new(text_brush, device),
        }
    }
}

pub struct ShashlikRenderer {
    world_tree_node: SceneTree,
    layers: Layers,
    depth_texture: DepthTexture,
    msaa_texture: MultisampledTexture,
    canvas: Box<dyn WgpuCanvas>,
    renderer_rx: Receiver<RendererMessage>,
    pub api: Arc<RendererApi>,
    global_context: GlobalContext,
}

impl ShashlikRenderer {
    pub async fn new(
        camera_controller: Rc<RefCell<CameraController>>,
        canvas: Box<dyn WgpuCanvas>,
    ) -> anyhow::Result<ShashlikRenderer> {
        let device = canvas.device();
        let config = canvas.config();

        let mut world_tree_node = SceneTree::new(World::new(), "".to_string());

        let camera_node = world_tree_node.add_child(CameraNode::new(&config, &device));

        let depth_texture = DepthTexture::new(&device, config.width, config.height);
        let msaa_texture =
            MultisampledTexture::new(device, config.width, config.height, config.format);
        let depth_state = DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: false,
            depth_compare: CompareFunction::Less,
            stencil: Default::default(),
            bias: Default::default(),
        };
        let multisample_state = wgpu::MultisampleState {
            count: MultisampledTexture::SAMPLE_COUNT,
            mask: !0,
            alpha_to_coverage_enabled: false,
        };

        let global_context = GlobalContext::new(
            camera_controller.clone(),
            CollisionHandler::new(),
            create_default_text_brush(
                device,
                config,
                depth_state.clone(),
                multisample_state.clone(),
            ),
            device,
        );
        let pipeline_provider = PipeLineProvider::new(
            config.format,
            depth_state.clone(),
            multisample_state.clone(),
        );

        let style_store = StyleStore::new();

        let shape_layers = ShapeLayers::new(
            device,
            pipeline_provider.clone(),
            &style_store,
            camera_node.clone(),
        );

        let mesh_layer = camera_node.borrow_mut().add_child_with_key(
            MeshLayer::new(
                &device,
                include_wgsl!("shaders/mesh_shader.wgsl"),
                Rc::new([VertexNormal::desc(), InstancePos::desc()]),
                pipeline_provider.clone(),
                Some(Face::Front),
            ),
            "mesh layer".to_string(),
        );

        let screen_shape_layer = MeshLayer::new(
            &device,
            include_wgsl!("shaders/screen_shape_shader.wgsl"),
            Rc::new([ShapeVertex::desc(), InstancePos::desc()]),
            pipeline_provider.clone(),
            None,
        );

        // TODO Why does it need a specific CompareFunction while e.g. FpsNode doesn't need it to be on top of screen?
        let screen_shape_layer: StyleAdapterNode<MeshLayer> = StyleAdapterNode::new(
            device,
            style_store.subscribe(),
            screen_shape_layer,
            SHADER_STYLE_GROUP_INDEX,
            CompareFunction::Always,
        );

        let screen_shape_layer = camera_node
            .borrow_mut()
            .add_child_with_key(screen_shape_layer, "screen shape".to_string());

        let text_layer = camera_node.borrow_mut().add_child_with_key(
            MeshLayer::new(
                &device,
                include_wgsl!("shaders/mesh_shader.wgsl"),
                Rc::new([VertexNormal::desc(), InstancePos::desc()]),
                pipeline_provider.clone(),
                Some(Face::Front),
            ),
            "text_layer".to_string(),
        );

        // FIXME
        // text_layer.borrow_mut().add_child_with_key(
        //     FpsNode::new(create_default_text_brush(
        //         device,
        //         config,
        //         depth_state.clone(),
        //         multisample_state.clone(),
        //     )),
        //     "fps_node".to_string(),
        // );

        let mut render_context = RenderContext::default();
        world_tree_node.setup(&mut render_context, &device);

        let (renderer_api_tx, renderer_api_rx) = channel();

        let (renderer_tx, renderer_rx) = channel();
        Self::run_background(style_store, renderer_tx.clone(), renderer_api_rx);

        let api = Arc::new(RendererApi::new(renderer_api_tx));

        Ok(Self {
            world_tree_node,
            layers: Layers::new(
                shape_layers,
                mesh_layer,
                screen_shape_layer,
                text_layer),
            depth_texture,
            msaa_texture,
            canvas,
            renderer_rx,
            api,
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
                    RendererApiMsg::RenderGroup((key, layer, spatial_data, mut rg)) => {
                        let (spatial_tx, _) = broadcast::channel(1);
                        spatial_data_map
                            .insert(key.clone(), (spatial_data.clone(), spatial_tx.clone()));

                        canvas_api.begin_shape(layer);
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

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.canvas.on_resize(width, height);
            let config = self.canvas.config();
            let device = self.canvas.device();
            let queue = self.canvas.queue();
            self.world_tree_node
                .resize(config.width, config.height, queue);
            self.depth_texture = DepthTexture::new(&device, config.width, config.height);
            self.msaa_texture =
                MultisampledTexture::new(device, config.width, config.height, config.format);

            self.global_context.text_renderer.text_brush.resize_view(
                config.width as f32,
                config.height as f32,
                queue,
            );
        }
    }

    pub fn update_and_render(&mut self) -> Result<(), SurfaceError> {
        self.update();
        self.render()
    }

    fn update(&mut self) {
        let device = self.canvas.device();
        if let Ok(message) = self.renderer_rx.try_recv() {
            match message {
                RendererMessage::Draw(mut draw_commands) => {
                    draw_commands.execute(
                        &device,
                        &mut self.layers
                    );
                }
                RendererMessage::ClearGroups(keys) => {
                    keys.into_iter().for_each(|key| {
                        self.layers.clear(key);
                    });
                }
            }
        }

        let queue = self.canvas.queue();
        let device = self.canvas.device();
        let config = self.canvas.config();
        self.world_tree_node
            .update(device, queue, config, &mut self.global_context);

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
            });

            self.world_tree_node
                .render(&mut render_pass, &mut self.global_context);

            self.global_context
                .text_renderer
                .render(&queue, &device, &mut render_pass)
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

    fn update_and_render(&mut self) -> Result<(), SurfaceError> {
        self.update_and_render()
    }

    fn update(&mut self) {
        self.update();
    }

    fn render(&mut self) -> Result<(), SurfaceError> {
        self.render()
    }
}
