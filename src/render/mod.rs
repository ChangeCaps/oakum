mod camera;
mod phase;
mod shader;

use std::num::NonZeroU32;

pub use camera::*;
pub use phase::*;
pub use shader::*;

use anyhow::bail;

use crate::world::World;

pub async unsafe fn init_wgpu_async(
    window: &winit::window::Window,
) -> anyhow::Result<(wgpu::Surface, wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });
    let surface = instance.create_surface(window)?;

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .ok_or(anyhow::anyhow!("No suitable adapter found"))?;

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits {
                    max_texture_dimension_1d: 4096,
                    max_texture_dimension_2d: 4096,
                    max_texture_dimension_3d: 4096,
                    ..wgpu::Limits::default()
                },
            },
            None,
        )
        .await?;

    Ok((surface, device, queue))
}

pub unsafe fn init_wgpu(
    window: &winit::window::Window,
) -> anyhow::Result<(wgpu::Surface, wgpu::Device, wgpu::Queue)> {
    hyena::block_on(init_wgpu_async(window))
}

#[derive(Clone, Copy)]
pub struct RenderContext<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub surface: &'a wgpu::Surface,
    pub texture: &'a wgpu::Texture,
    pub view: &'a wgpu::TextureView,
    pub hdr_texture: &'a wgpu::Texture,
    pub hdr_view: &'a wgpu::TextureView,
    pub depth_texture: &'a wgpu::Texture,
    pub depth_view: &'a wgpu::TextureView,
    pub world: &'a World,
    pub camera: &'a DrawCamera,
    pub width: u32,
    pub height: u32,
    pub taa_sample: u32,
    pub taa_samples: u32,
}

pub struct Renderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub needs_configure: bool,
    pub hdr_texture: wgpu::Texture,
    pub depth_texture: wgpu::Texture,
    pub camera: DrawCamera,
    pub octree_phase: OctreePhase,
    pub tonemap_phase: TonemapPhase,
    pub taa_sample: u32,
    pub taa_samples: u32,
}

impl Renderer {
    pub const HDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub unsafe fn new(window: &winit::window::Window) -> anyhow::Result<Self> {
        let (surface, device, queue) = init_wgpu(window)?;

        let width = window.inner_size().width;
        let height = window.inner_size().height;

        let taa_samples = 2;

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::STORAGE_BINDING,
            format: wgpu::TextureFormat::Bgra8Unorm,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            view_formats: vec![],
        };

        let hdr_texture = Self::create_hdr_texture(&device, width, height, taa_samples);
        let hdr_view = hdr_texture.create_view(&Default::default());

        let depth_texture = Self::create_depth_texture(&device, width, height);

        let camera = DrawCamera::new(&device)?;
        let octree_phase = OctreePhase::new(&device, &camera)?;
        let tonemap_phase = TonemapPhase::new(&device, &hdr_view)?;

        Ok(Self {
            device,
            queue,
            surface,
            surface_config,
            needs_configure: true,
            hdr_texture,
            depth_texture,
            camera,
            octree_phase,
            tonemap_phase,
            taa_sample: 0,
            taa_samples,
        })
    }

    fn create_hdr_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        taa_samples: u32,
    ) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: taa_samples,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::HDR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    }

    fn create_depth_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;

        self.needs_configure = true;
    }

    pub fn configure(&mut self) {
        self.needs_configure = false;

        self.surface.configure(&self.device, &self.surface_config);

        let width = self.surface_config.width;
        let height = self.surface_config.height;
        self.hdr_texture = Self::create_hdr_texture(&self.device, width, height, self.taa_samples);
        self.depth_texture = Self::create_depth_texture(&self.device, width, height);

        let hdr_view = self.hdr_texture.create_view(&Default::default());
        self.tonemap_phase.resized(&self.device, &hdr_view);
    }

    pub fn aspect(&self) -> f32 {
        self.surface_config.width as f32 / self.surface_config.height as f32
    }

    pub fn render_frame(&mut self, world: &World) -> anyhow::Result<()> {
        if self.needs_configure {
            self.configure();
        }

        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.surface_config);
                self.surface.get_current_texture()?
            }
            Err(wgpu::SurfaceError::Timeout) => return Ok(()),
            Err(e) => bail!(e),
        };

        let view = frame.texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());

        (self.camera).write(&self.queue, &world.camera, self.aspect());

        self.main_pass(&mut encoder, &frame.texture, &view, world)?;

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();

        Ok(())
    }

    pub fn main_pass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        texture: &wgpu::Texture,
        view: &wgpu::TextureView,
        world: &World,
    ) -> anyhow::Result<()> {
        let hdr_view = self.hdr_texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("hdr_view"),
            dimension: Some(wgpu::TextureViewDimension::D2),
            base_array_layer: self.taa_sample,
            array_layer_count: NonZeroU32::new(1),
            ..Default::default()
        });
        let depth_view = self.depth_texture.create_view(&Default::default());

        let cx = RenderContext {
            device: &self.device,
            queue: &self.queue,
            surface: &self.surface,
            texture,
            view,
            world,
            hdr_texture: &self.hdr_texture,
            hdr_view: &hdr_view,
            depth_texture: &self.depth_texture,
            depth_view: &depth_view,
            camera: &self.camera,
            width: self.surface_config.width,
            height: self.surface_config.height,
            taa_sample: self.taa_sample,
            taa_samples: self.taa_samples,
        };

        self.octree_phase.render(encoder, cx)?;
        self.tonemap_phase.render(encoder, cx)?;

        self.taa_sample = (self.taa_sample + 1) % self.taa_samples;

        Ok(())
    }

    pub fn octree_phase(&self) -> &OctreePhase {
        &self.octree_phase
    }
}
