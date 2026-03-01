use crate::collider::Collider;
use crate::consts::STYLE_SHADER_PARAMS_COUNT;
use crate::styles::style_store::StyleStore;
use crate::utils::ReceiverExt;
use crate::view_projection::ViewProjection;
use cgmath::{Matrix4, Vector3};
use wgpu::util::{DeviceExt, DrawIndexedIndirectArgs};
use wgpu::{BindGroup, BindGroupLayout, Buffer, Device};
use wgpu_canvas::wgpu_canvas::WgpuCanvas;
use crate::mesh::mesh_instance_input::MeshInstanceInput;

pub struct GlobalContext {
    pub canvas: Box<dyn WgpuCanvas>,
    pub view_projection: ViewProjection,
    pub collider: Collider,
    pub styles_bind_group_layout: BindGroupLayout,
    pub style_bind_group: Option<BindGroup>,
    pub kiol_data: (BindGroupLayout, BindGroup, BindGroupLayout, BindGroup, BindGroupLayout, BindGroup),
    pub dots: usize,
    pub indirect_args: Buffer,
    style_uniform_rx: tokio::sync::broadcast::Receiver<Vec<[f32; STYLE_SHADER_PARAMS_COUNT]>>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DotInput {
    pub position: [f32; 3],
    pub color_alpha: f32,
}

impl GlobalContext {
    pub fn new(canvas: Box<dyn WgpuCanvas>, style_store: &StyleStore) -> Self {
        let device = canvas.device();
        let view_projection = ViewProjection::new(device);
        let collider = Collider::new();
        let styles_bind_group_layout = Self::create_style_bind_group_layout(device);

        let indirect_args_struct = DrawIndexedIndirectArgs {
            index_count: 6,
            instance_count: 0,
            first_index: 0,
            base_vertex: 0,
            first_instance: 0,
        };
        let indirect_args = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("indirect args"),
            contents: indirect_args_struct.as_bytes(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDIRECT |wgpu::BufferUsages::COPY_DST,
        });

        let kiol_data = Self::create_route_render_data(&device, &view_projection, &indirect_args);

        GlobalContext {
            canvas,
            view_projection,
            collider,
            styles_bind_group_layout,
            style_bind_group: None,
            kiol_data,
            dots: 0,
            indirect_args,
            style_uniform_rx: style_store.subscribe(),
        }
    }

    fn create_style_bind_group_layout(device: &Device, ) -> BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("styles_bind_group_layout"),
        })
    }

    fn create_route_render_data(device: &Device, vp: &ViewProjection, indirect_args: &Buffer) -> (BindGroupLayout, BindGroup, BindGroupLayout, BindGroup, BindGroupLayout, BindGroup) {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("kiol compute Buffer"),
            contents: bytemuck::cast_slice(&[0.0, 0.0, 0.0, 0.0]),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let culled_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("kiol culled Buffer"),
            contents: bytemuck::cast_slice(&[0]),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let layout1 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            label: Some("compute_bind_group_layout1"),
        });
        let layout2 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }, wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }, wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("compute_bind_group_layout2"),
        });
        let layout3 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("compute_bind_group_layout2"),
        });

        let bind_group1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout1,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: culled_buffer.as_entire_binding(),
                }],
            label: Some("styles_bind_group1"),
        });
        let bind_group2 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout2,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: culled_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: indirect_args.as_entire_binding(),
                }],
            label: Some("styles_bind_group2"),
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout3,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: vp
                    .uniform_buffer
                    .as_entire_binding(),
            }],
            label: Some("camera_bind_group_compute"),
        });
        (layout1, bind_group1, layout2, bind_group2, layout3, bind_group )
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.canvas.on_resize();
            let config = self.canvas.config();

            self.view_projection.resize(config.width, config.height);
        }
    }

    pub fn set_route_dots<T: MeshInstanceInput>(&mut self, positions: &Vec<T>) {
        println!("kiol set_route_dots");
        let dot_input_vec: Vec<_> = positions.iter().map(|item| {
            DotInput {
                position: item.position(),
                color_alpha: 0.0,
            }
        }).collect();
        let count = positions.len();
        println!("kiol count = {count}");
        let device = self.device();
        let buffer = self.device().create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("kiol compute Buffer"),
            contents: bytemuck::cast_slice(dot_input_vec.as_slice()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let culled_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("kiol culled Buffer"),
            contents: bytemuck::cast_slice(&vec![0; dot_input_vec.len()]),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.kiol_data.0,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: culled_buffer.as_entire_binding(),
                }, ],
            label: Some("kiol_styles_bind_group1"),
        });
        let bind_group2 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.kiol_data.2,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: culled_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.indirect_args.as_entire_binding(),
                }],
            label: Some("kiol_styles_bind_group2"),
        });
        self.kiol_data.1 = bind_group1;
        self.kiol_data.3 = bind_group2;
        self.dots = count;
    }

    pub fn update(&mut self, view_proj_matrix: Matrix4<f64>, cs_offset: Vector3<f64>, scale: f32) {
        self.view_projection.update(
            self.canvas.queue(),
            self.canvas.config(),
            view_proj_matrix,
            cs_offset,
            scale
        );
        self.collider.update_view_proj(&self.view_projection);

        self.update_style_bind_group();
    }

    fn update_style_bind_group(&mut self) {
        let device = self.canvas.device();
        if let Ok(uniforms) = self.style_uniform_rx.no_lagged() {
            // TODO We could reuse the buffer if styles count has not changed
            let styles_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Style Buffer"),
                contents: bytemuck::cast_slice(&uniforms),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

            let styles_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.styles_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: styles_buffer.as_entire_binding(),
                }],
                label: Some("styles_bind_group"),
            });

            self.style_bind_group = Some(styles_bind_group);
        }
    }

    pub fn queue(&self) -> &wgpu::Queue {
        self.canvas.queue()
    }
    pub fn config(&self) -> &wgpu::SurfaceConfiguration {
        self.canvas.config()
    }
    pub fn device(&self) -> &wgpu::Device {
        self.canvas.device()
    }
}