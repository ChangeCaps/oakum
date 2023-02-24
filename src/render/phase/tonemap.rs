use crate::render::{open_shader, RenderContext};

pub struct TonemapPipeline {
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub layout: wgpu::PipelineLayout,
    pub pipeline: wgpu::RenderPipeline,
}

impl TonemapPipeline {
    pub fn new(device: &wgpu::Device) -> anyhow::Result<Self> {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Tonemap Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                    multisampled: false,
                },
                count: None,
            }],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Tonemap Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let vertex_shader = open_shader(device, "assets/shaders/fullscreen.wgsl")?;
        let fragment_shader = open_shader(device, "assets/shaders/tonemap.wgsl")?;

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Tonemap Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &vertex_shader,
                entry_point: "main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &fragment_shader,
                entry_point: "main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: Default::default(),
            multisample: Default::default(),
            multiview: Default::default(),
        });

        Ok(Self {
            bind_group_layout,
            layout,
            pipeline,
        })
    }
}

pub struct TonemapPhase {
    pub pipeline: TonemapPipeline,
    pub bind_group: wgpu::BindGroup,
}

impl TonemapPhase {
    pub fn new(device: &wgpu::Device, hdr_view: &wgpu::TextureView) -> anyhow::Result<Self> {
        let pipeline = TonemapPipeline::new(device)?;

        let bind_group = Self::create_bind_group(&pipeline, device, hdr_view);

        Ok(Self {
            pipeline,
            bind_group,
        })
    }

    fn create_bind_group(
        pipeline: &TonemapPipeline,
        device: &wgpu::Device,
        hdr_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Tonemap Bind Group"),
            layout: &pipeline.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(hdr_view),
            }],
        })
    }

    pub fn resized(&mut self, device: &wgpu::Device, hdr_view: &wgpu::TextureView) {
        self.bind_group = Self::create_bind_group(&self.pipeline, device, hdr_view);
    }

    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        cx: RenderContext,
    ) -> anyhow::Result<()> {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Tonemap Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &cx.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        pass.set_pipeline(&self.pipeline.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..6, 0..1);

        Ok(())
    }
}
