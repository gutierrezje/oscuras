use bytemuck::cast_slice;
use cgmath::Vector3;
use wgpu::{ShaderStage, util::DeviceExt};

use super::camera::Camera;
use super::data_types::*;
use super::scene::Scene;
use super::gpu_buffer::{GPUBufferDescription, GPUBuffer};

pub struct Pathtracer {
    width: u32,
    height: u32,
    // Resources
    display_texture: wgpu::Texture,
    display_sampler: wgpu::Sampler,
    intersect_buffer: GPUBuffer,
    geometry_buffer: GPUBuffer,
    camera_buffer: GPUBuffer,
    paths_buffer: GPUBuffer,
    params_buffer0: GPUBuffer,
    params_buffer1: GPUBuffer,

    // Pipelines
    path_gen_bg: wgpu::BindGroup,
    path_gen_pipeline: wgpu::ComputePipeline,
    hit_calc_bg: wgpu::BindGroup,
    hit_calc_pipeline: wgpu::ComputePipeline,
    image_bg: wgpu::BindGroup,
    image_pipeline: wgpu::ComputePipeline,
}

impl Pathtracer {
    pub fn new(device: &wgpu::Device, camera: &Camera, scene: &Scene) -> Self {
        let width = camera.res_x();
        let height = camera.res_y();

        // Initialize resources
        let temp_cam = [*camera];
        let camera_buf_desc = GPUBufferDescription::<Camera> {
            binding_type: wgpu::BufferBindingType::Uniform,
            contents: Some(&temp_cam),
            element_count: 1,
            element_size: std::mem::size_of::<Camera>(),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        };
        let camera_buffer = GPUBuffer::new(&device, camera_buf_desc);

        let paths_buf_desc = GPUBufferDescription::<()> {
            binding_type: wgpu::BufferBindingType::Storage {read_only: true},
            contents: None,
            element_count: width * height,
            element_size: std::mem::size_of::<Ray>(),
            usage: wgpu::BufferUsage::STORAGE,
        };
        let paths_buffer = GPUBuffer::new(&device, paths_buf_desc);

        let params0 = [width, height];
        let params_buf0_des = GPUBufferDescription::<u32> {
            binding_type: wgpu::BufferBindingType::Uniform,
            contents: Some(&params0),
            element_count: 2,
            element_size: 4,
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        };
        let params_buffer0 = GPUBuffer::new(&device, params_buf0_des);

        let intersect_buf_desc = GPUBufferDescription::<()> {
            binding_type: wgpu::BufferBindingType::Storage {read_only: true},
            contents: None,
            element_count: width * height,
            element_size: std::mem::size_of::<u32>(),
            usage: wgpu::BufferUsage::STORAGE,
        };
        let intersect_buffer = GPUBuffer::new(&device, intersect_buf_desc);

        let geom_buf_desc = GPUBufferDescription::<Sphere> {
            binding_type: wgpu::BufferBindingType::Storage {read_only: true},
            contents: Some(&scene.geometry),
            element_count: scene.geometry.len() as u32,
            element_size: std::mem::size_of::<Sphere>(),
            usage: wgpu::BufferUsage::STORAGE,
        };
        let geometry_buffer = GPUBuffer::new(&device, geom_buf_desc);

        let params1 = [scene.geometry.len() as u32, width * height];
        let params_buf1_desc = GPUBufferDescription::<u32> {
            binding_type: wgpu::BufferBindingType::Uniform,
            contents: Some(&params1),
            element_count: 2,
            element_size: 4,
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        };
        let params_buffer1 = GPUBuffer::new(&device, params_buf1_desc);
        //let params_buffer1 = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //    label: Some("params_buffer1"),
        //    contents: bytemuck::cast_slice(&[scene.geometry.len() as u32, width * height]),
        //    usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        //});

        let texture_size = wgpu::Extent3d {
            width,
            height,
            depth: 1,
        };
        let display_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsage::SAMPLED
                | wgpu::TextureUsage::COPY_DST
                | wgpu::TextureUsage::STORAGE,
            label: Some("display_texture"),
        });
        let display_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let path_gen_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                camera_buffer.as_bgl_entry(0, wgpu::ShaderStage::COMPUTE, true),
                paths_buffer.as_bgl_entry(1, wgpu::ShaderStage::COMPUTE, false),
            ],
        });
        let path_gen_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("path_gen_bind_group"),
            layout: &path_gen_bgl,
            entries: &[
                camera_buffer.as_bg_entry(0),
                paths_buffer.as_bg_entry(1),
            ],
        });

        // Getting around https://github.com/gfx-rs/naga/issues/406
        let path_gen_module = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("../shaders/generate_paths.comp.spv"),
            source: wgpu::util::make_spirv(include_bytes!("../shaders/generate_paths.comp.spv")),
            flags: std::iter::empty::<wgpu::ShaderFlags>().collect(),
        });
        //let path_gen_module = device.create_shader_module(&wgpu::include_spirv!("../shaders/generate_paths.comp.spv"));
        let path_gen_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&path_gen_bgl],
            push_constant_ranges: &[],
        });
        let path_gen_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("path_gen_pipeline"),
            layout: Some(&path_gen_pl),
            module: &path_gen_module,
            entry_point: "main",
        });

        let hit_calc_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                intersect_buffer.as_bgl_entry(0, wgpu::ShaderStage::COMPUTE, false),
                geometry_buffer.as_bgl_entry(1, wgpu::ShaderStage::COMPUTE, true),
                paths_buffer.as_bgl_entry(2, wgpu::ShaderStage::COMPUTE, true),
                params_buffer1.as_bgl_entry(3, wgpu::ShaderStage::COMPUTE, true),
            ],
        });

        let hit_calc_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &hit_calc_bgl,
            entries: &[
                intersect_buffer.as_bg_entry(0),
                geometry_buffer.as_bg_entry(1),
                paths_buffer.as_bg_entry(2),
                params_buffer1.as_bg_entry(3),
            ],
        });

        let hit_calc_module = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::util::make_spirv(include_bytes!(
                "../shaders/calculate_intersections.comp.spv"
            )),
            flags: std::iter::empty::<wgpu::ShaderFlags>().collect(),
        });

        let hit_calc_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&hit_calc_bgl],
            push_constant_ranges: &[],
        });

        let hit_calc_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("hit_calc_pipeline"),
            layout: Some(&hit_calc_pl),
            module: &hit_calc_module,
            entry_point: "main",
        });

        let image_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::COMPUTE,
                    ty: wgpu::BindingType::Sampler {
                        filtering: true,
                        comparison: false,
                    },
                    count: None,
                },
                paths_buffer.as_bgl_entry(2, wgpu::ShaderStage::COMPUTE, true),
                params_buffer0.as_bgl_entry(3, wgpu::ShaderStage::COMPUTE, true),
                intersect_buffer.as_bgl_entry(4, wgpu::ShaderStage::COMPUTE, true),
            ],
        });
        let image_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("image_bind_group"),
            layout: &image_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &display_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&display_sampler),
                },
                paths_buffer.as_bg_entry(2),
                params_buffer0.as_bg_entry(3),
                intersect_buffer.as_bg_entry(4),
            ],
        });

        let image_comp_module =
            device.create_shader_module(&wgpu::include_spirv!("../shaders/viewer.comp.spv"));
        let image_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&image_bgl],
            push_constant_ranges: &[],
        });
        let image_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("image_pipeline"),
            layout: Some(&image_pl),
            module: &image_comp_module,
            entry_point: "main",
        });

        Self {
            width,
            height,
            display_texture,
            display_sampler,
            path_gen_bg,
            path_gen_pipeline,
            hit_calc_bg,
            hit_calc_pipeline,
            image_bg,
            image_pipeline,
            camera_buffer,
            paths_buffer,
            params_buffer0,
            params_buffer1,
            intersect_buffer,
            geometry_buffer,
        }
    }

    pub fn run(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut compute_encoder =
            encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });

        let block_dims_2d = Vector3::new(
            (self.width as f32 / 16_f32).ceil() as u32,
            (self.height as f32 / 16_f32).ceil() as u32,
            1,
        );

        let block_dims_1d = Vector3::new(
            ((self.width * self.height) as f32 / 256f32).ceil() as u32,
            1,
            1,
        );

        compute_encoder.set_pipeline(&self.path_gen_pipeline);
        compute_encoder.set_bind_group(0, &self.path_gen_bg, &[]);
        compute_encoder.dispatch(block_dims_2d.x, block_dims_2d.y, block_dims_2d.z);

        compute_encoder.set_pipeline(&self.hit_calc_pipeline);
        compute_encoder.set_bind_group(0, &self.hit_calc_bg, &[]);
        compute_encoder.dispatch(block_dims_1d.x, block_dims_1d.y, block_dims_1d.z);

        compute_encoder.set_pipeline(&self.image_pipeline);
        compute_encoder.set_bind_group(0, &self.image_bg, &[]);
        compute_encoder.dispatch(block_dims_2d.x, block_dims_2d.y, block_dims_2d.z);
        drop(compute_encoder);
    }

    fn reset_resources(&self, camera: &Camera) {
        /* TODO: Upon implementing camera movement
        Need to update the following:
        The resolution in params_buffer
        Reinitialize camera_buffer
        Reinitialize paths_buffer
        */
        todo!()
    }

    pub fn texture(&self) -> &wgpu::Texture {
        &self.display_texture
    }

    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.display_sampler
    }
}
