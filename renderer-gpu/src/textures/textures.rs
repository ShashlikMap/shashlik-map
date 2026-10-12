use crate::global_context::GlobalContext;
use std::borrow::Cow;
use std::fs;
use wgpu::util::DeviceExt;
use wgpu::{AstcBlock, AstcChannel, Device, Extent3d, Queue, TextureView};

pub(crate) struct Textures {
    stars_texture: TextureView,
}

impl Textures {
    const STARS_BYTES: &'static [u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../stars-cubemap/astc.ktx2"
    ));

    pub fn new(global_context: &GlobalContext) -> Self {
        Self {
            stars_texture: Self::create_stars_texture(global_context.device(), global_context.queue()),
        }
    }

    pub fn get_stars_texture(&self) -> &TextureView {
        &self.stars_texture
    }

    fn create_stars_texture(device: &Device, queue: &Queue) -> TextureView {
        let star_box_format = if device
            .features()
            .contains(wgpu::Features::TEXTURE_COMPRESSION_ASTC)
        {
            wgpu::TextureFormat::Astc {
                block: AstcBlock::B4x4,
                channel: AstcChannel::UnormSrgb,
            }
        } else {
            wgpu::TextureFormat::Bc7RgbaUnormSrgb
        };

        // fyi, BC7 is used only as fallback if ASTC is not supported. So far the only case is headless mode that is compiled from source
        // and texture file is always available
        let stars_bytes = if star_box_format == wgpu::TextureFormat::Bc7RgbaUnormSrgb {
            Cow::Owned(
                fs::read(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../stars-cubemap/bc7.ktx2"
                ))
                .unwrap(),
            )
        } else {
            Cow::Borrowed(Self::STARS_BYTES)
        };

        let reader = ktx2::Reader::new(stars_bytes).unwrap();
        let header = reader.header();
        let mut image = Vec::with_capacity(reader.data().len());
        for level in reader.levels() {
            image.extend_from_slice(level.data);
        }

        let texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                size: Extent3d {
                    width: header.pixel_width,
                    height: header.pixel_height,
                    depth_or_array_layers: header.face_count,
                },
                mip_level_count: header.level_count,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: star_box_format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                label: None,
                view_formats: &[],
            },
            // KTX2 stores mip levels in mip major order.
            wgpu::util::TextureDataOrder::MipMajor,
            &image,
        );

        texture.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..wgpu::TextureViewDescriptor::default()
        })
    }
}
