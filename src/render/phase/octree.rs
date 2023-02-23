use std::{mem, num::NonZeroU32};

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use log::{debug, trace};

use crate::{
    octree::{DynamicOctree, Node, Segment},
    render::{open_shader, DrawCamera, RenderContext, Renderer},
};

pub struct OctreePipeline {
    pub uniform_layout: wgpu::BindGroupLayout,
    pub light_layout: wgpu::BindGroupLayout,
    pub octree_layout: wgpu::BindGroupLayout,
    pub layout: wgpu::PipelineLayout,
    pub compute_pipeline: wgpu::ComputePipeline,
}

impl OctreePipeline {
    pub fn new(device: &wgpu::Device) -> anyhow::Result<Self> {
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform Bind Group Layout"),
            entries: &[
                // camera
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: Renderer::HDR_FORMAT,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let light_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Light Bind Group Layout"),
            entries: &[],
        });

        let octree_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Octree Bind Group Layout"),
            entries: &[
                // octree
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Uint,
                        view_dimension: wgpu::TextureViewDimension::D3,
                        multisampled: false,
                    },
                    count: None,
                },
                // octree uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Octree Pipeline Layout"),
            bind_group_layouts: &[&uniform_layout, &light_layout, &octree_layout],
            push_constant_ranges: &[],
        });

        /*
        let vertex_shader = open_shader(device, "assets/shaders/pbr_vert.wgsl")?;
        let fragment_shader = open_shader(device, "assets/shaders/pbr_frag.wgsl")?;

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Octree Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                entry_point: "main",
                module: &vertex_shader,
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                entry_point: "main",
                module: &fragment_shader,
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: Default::default(),
            multisample: Default::default(),
            multiview: None,
        });
        */

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Octree Pipeline"),
            layout: Some(&layout),
            module: &open_shader(device, "assets/shaders/pbr_comp.wgsl")?,
            entry_point: "main",
        });

        Ok(Self {
            uniform_layout,
            light_layout,
            octree_layout,
            layout,
            compute_pipeline,
        })
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct OctreeUniform {
    pub model: Mat4,
    pub model_inv: Mat4,
}

pub struct DrawOctree {
    /// The octree is stored in a 2d texture array,
    /// where each layer is a page of the octree.
    ///
    /// Indices are encoded as follows:
    /// | 12 | 12 |  8   |
    /// |----|----|------|
    /// |  x |  y | page |
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    /// The height of each page in the octree.
    pub page_height: u32,
    /// The number of pages in the octree.
    pub page_count: u32,
    /// The uniform buffer for the octree.
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl DrawOctree {
    pub const PAGE_SIZE: u32 = 1 << 12;

    pub fn new(device: &wgpu::Device, pipeline: &OctreePipeline) -> anyhow::Result<Self> {
        let page_height = 1;
        let page_count = 1;

        let texture = Self::create_texture(device, page_height, page_count);
        let view = texture.create_view(&Default::default());

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Octree Uniform Buffer"),
            size: mem::size_of::<OctreeUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = Self::create_bind_group(device, pipeline, &view, &uniform_buffer);

        Ok(Self {
            texture,
            view,
            page_height,
            page_count,
            uniform_buffer,
            bind_group,
        })
    }

    /// Returns the number of nodes that can be stored in the texture.
    pub const fn size(&self) -> u64 {
        Self::PAGE_SIZE as u64 * self.page_height as u64 * self.page_count as u64
    }

    /// Returns the size of the texture in bytes.
    pub const fn byte_size(&self) -> u64 {
        self.size() * mem::size_of::<Node>() as u64
    }

    /// Returns the size of a page in nodes.
    pub const fn page_size(&self) -> u32 {
        Self::PAGE_SIZE * self.page_height
    }

    /// Returns the size of a page in bytes.
    pub const fn bytes_per_page(&self) -> u64 {
        self.page_size() as u64 * mem::size_of::<Node>() as u64
    }

    /// Returns the number of bytes in a row of the texture.
    pub const fn bytes_per_row(&self) -> u32 {
        Self::PAGE_SIZE * mem::size_of::<Node>() as u32
    }

    fn create_texture(device: &wgpu::Device, height: u32, page_count: u32) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Octree Texture"),
            size: wgpu::Extent3d {
                width: Self::PAGE_SIZE,
                height,
                depth_or_array_layers: page_count,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::Rg32Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
    }

    fn create_bind_group(
        device: &wgpu::Device,
        pipeline: &OctreePipeline,
        view: &wgpu::TextureView,
        uniform_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Octree Bind Group"),
            layout: &pipeline.octree_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        })
    }

    /// Resize the octree texture.
    ///
    /// - `size` is the number of nodes that can be stored in the texture.
    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pipeline: &OctreePipeline,
        size: u64,
    ) {
        if self.size() >= size {
            return;
        }

        let old_page_height = self.page_height;
        let old_page_count = self.page_count;

        while self.size() < size {
            if self.page_height < Self::PAGE_SIZE {
                self.page_height *= 2;
            } else {
                self.page_count += 1;
            }
        }

        trace!(
            "Resizing octree texture to {}x{}x{}, taking up {}Gb",
            Self::PAGE_SIZE,
            self.page_height,
            self.page_count,
            self.byte_size() as f64 / 1024.0 / 1024.0 / 1024.0,
        );

        let texture = Self::create_texture(device, self.page_height, self.page_count);

        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: Self::PAGE_SIZE,
                height: old_page_height,
                depth_or_array_layers: old_page_count,
            },
        );
        queue.submit(Some(encoder.finish()));

        self.texture = texture;
        self.view = self.texture.create_view(&Default::default());
        self.bind_group =
            Self::create_bind_group(device, pipeline, &self.view, &self.uniform_buffer);
    }

    pub fn write_uniform(&self, queue: &wgpu::Queue, model: Mat4) {
        let uniform = OctreeUniform {
            model,
            model_inv: model.inverse(),
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniform));
    }

    /// Write changes from a [`DynamicOctree`] to the texture.
    pub fn write_dynamic(&self, queue: &wgpu::Queue, octree: &DynamicOctree) {
        for &segment in octree.segments() {
            assert!(segment.byte_end() <= octree.size());

            self.write_segment(queue, segment, octree.bytes());
        }
    }

    fn write_first_row(
        &self,
        queue: &wgpu::Queue,
        offset: &mut usize,
        size: &mut usize,
        row: &mut usize,
        page: usize,
        bytes: &[u8],
    ) {
        let row_offset = *offset % self.bytes_per_row() as usize;
        if row_offset > 0 {
            let row_size = usize::min(self.bytes_per_row() as usize - row_offset, *size);
            self.write_row(
                queue,
                row_offset as u32,
                *row as u32,
                page as u32,
                &bytes[*offset..*offset + row_size],
            );

            *row += 1;
            *offset += row_size;
            *size -= row_size;
        }
    }

    fn write_first_rows(
        &self,
        queue: &wgpu::Queue,
        offset: &mut usize,
        size: &mut usize,
        row: &mut usize,
        page: usize,
        bytes: &[u8],
    ) {
        let page_offset = *row % self.page_height as usize;
        let rows = usize::min(
            self.page_height as usize - page_offset,
            *size / self.bytes_per_row() as usize,
        );

        if rows > 0 {
            self.write_rows(
                queue,
                page_offset as u32,
                rows as u32,
                page as u32,
                &bytes[*offset..],
            );

            let written = rows * self.bytes_per_row() as usize;

            *row += rows;
            *offset += written;
            *size -= written;
        }
    }

    fn write_full_pages(
        &self,
        queue: &wgpu::Queue,
        offset: &mut usize,
        size: &mut usize,
        page: &mut usize,
        bytes: &[u8],
    ) {
        let pages = *size / self.bytes_per_page() as usize;
        if pages > 0 {
            self.write_pages(queue, *page as u32, pages as u32, &bytes[*offset..]);
            let written = pages * self.bytes_per_page() as usize;

            *page += pages;
            *offset += written;
            *size -= written;
        }
    }

    fn write_last_rows(
        &self,
        queue: &wgpu::Queue,
        offset: &mut usize,
        size: &mut usize,
        row: &mut usize,
        page: usize,
        bytes: &[u8],
    ) {
        let rows = *size / self.bytes_per_row() as usize;
        if rows > 0 {
            self.write_rows(queue, 0, rows as u32, page as u32, &bytes[*offset..]);

            let written = rows * self.bytes_per_row() as usize;

            *row += rows;
            *offset += written;
            *size -= written;
        }
    }

    fn write_last_row(
        &self,
        queue: &wgpu::Queue,
        offset: usize,
        size: usize,
        row: usize,
        page: usize,
        bytes: &[u8],
    ) {
        if size > 0 {
            let data = &bytes[offset..offset + size];
            self.write_row(queue, 0, row as u32, page as u32, data);
        }
    }

    fn write_segment(&self, queue: &wgpu::Queue, segment: Segment, bytes: &[u8]) {
        let mut offset = segment.byte_start();
        let mut size = segment.byte_len();

        let mut row = offset / self.bytes_per_row() as usize;
        let mut page = row / self.page_height as usize;

        self.write_first_row(queue, &mut offset, &mut size, &mut row, page, bytes);
        self.write_first_rows(queue, &mut offset, &mut size, &mut row, page, bytes);
        self.write_full_pages(queue, &mut offset, &mut size, &mut page, bytes);
        self.write_last_rows(queue, &mut offset, &mut size, &mut row, page, bytes);
        self.write_last_row(queue, offset, size, row, page, bytes);
    }

    fn write_row(&self, queue: &wgpu::Queue, offset: u32, row: u32, page: u32, bytes: &[u8]) {
        trace!("Writing row {} of page {} to octree texture", row, page);

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: offset / mem::size_of::<Node>() as u32,
                    y: row,
                    z: page,
                },
                aspect: wgpu::TextureAspect::All,
            },
            bytes,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: NonZeroU32::new(self.bytes_per_row() as u32),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: bytes.len() as u32 / mem::size_of::<Node>() as u32,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
    }

    pub fn write_rows(&self, queue: &wgpu::Queue, row: u32, rows: u32, page: u32, bytes: &[u8]) {
        trace!(
            "Writing {} rows starting at {} of page {} to octree texture",
            rows,
            row,
            page
        );

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: row,
                    z: page,
                },
                aspect: wgpu::TextureAspect::All,
            },
            bytes,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: NonZeroU32::new(self.bytes_per_row() as u32),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: Self::PAGE_SIZE,
                height: rows,
                depth_or_array_layers: 1,
            },
        );
    }

    fn write_pages(&self, queue: &wgpu::Queue, page: u32, pages: u32, bytes: &[u8]) {
        trace!(
            "Writing {} pages starting at {} to octree texture",
            pages,
            page
        );

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: page as u32,
                },
                aspect: wgpu::TextureAspect::All,
            },
            bytes,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: NonZeroU32::new(self.bytes_per_row() as u32),
                rows_per_image: NonZeroU32::new(self.page_height),
            },
            wgpu::Extent3d {
                width: Self::PAGE_SIZE,
                height: self.page_height,
                depth_or_array_layers: pages,
            },
        );
    }
}

pub struct OctreePhase {
    pub pipeline: OctreePipeline,
    pub taa_sample_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub light_bind_group: wgpu::BindGroup,
    pub draw_octree: DrawOctree,
}

impl OctreePhase {
    pub fn new(
        device: &wgpu::Device,
        camera: &DrawCamera,
        hdr_view: &wgpu::TextureView,
    ) -> anyhow::Result<Self> {
        let pipeline = OctreePipeline::new(device)?;

        let taa_sample_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("TAA Sample Buffer"),
            size: mem::size_of::<u32>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bind_group = Self::create_uniform_bind_group(
            &pipeline,
            device,
            camera,
            hdr_view,
            &taa_sample_buffer,
        );

        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Light Bind Group"),
            layout: &pipeline.light_layout,
            entries: &[],
        });

        let draw_octree = DrawOctree::new(device, &pipeline)?;

        Ok(Self {
            pipeline,
            taa_sample_buffer,
            light_bind_group,
            draw_octree,
            uniform_bind_group,
        })
    }

    fn create_uniform_bind_group(
        pipeline: &OctreePipeline,
        device: &wgpu::Device,
        camera: &DrawCamera,
        hdr_view: &wgpu::TextureView,
        taa_sample_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &pipeline.uniform_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(hdr_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: taa_sample_buffer.as_entire_binding(),
                },
            ],
        })
    }

    pub fn resized(
        &mut self,
        device: &wgpu::Device,
        camera: &DrawCamera,
        hdr_view: &wgpu::TextureView,
    ) {
        self.uniform_bind_group = Self::create_uniform_bind_group(
            &self.pipeline,
            device,
            camera,
            hdr_view,
            &self.taa_sample_buffer,
        );
    }

    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        cx: RenderContext,
    ) -> anyhow::Result<()> {
        (self.draw_octree).resize(
            cx.device,
            cx.queue,
            &self.pipeline,
            cx.world.octree.len() as u64,
        );
        (self.draw_octree).write_dynamic(cx.queue, &cx.world.octree);
        (self.draw_octree).write_uniform(cx.queue, Mat4::from_scale(Vec3::splat(10.0)));
        cx.queue.write_buffer(
            &self.taa_sample_buffer,
            0,
            bytemuck::bytes_of(&cx.taa_sample),
        );

        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&self.pipeline.compute_pipeline);
        pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        pass.set_bind_group(1, &self.light_bind_group, &[]);
        pass.set_bind_group(2, &self.draw_octree.bind_group, &[]);
        pass.dispatch_workgroups(cx.width / 16 + 1, cx.height / 16 + 1, 1);

        Ok(())
    }
}
