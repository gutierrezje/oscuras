pub struct Pathtracer {
    width: u32,
    height: u32,
    display_texture: wgpu::Texture,
    display_sampler: wgpu::Sampler,
    image_pipeline: wgpu::ComputePipeline,
    image_bind_group: wgpu::BindGroup,
}

impl Pathtracer {
    pub fn new(device: &wgpu::Device, resolution: winit::dpi::PhysicalSize<u32>) -> Self {
        let width = resolution.width;
        let height = resolution.height;

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
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST | wgpu::TextureUsage::STORAGE,
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

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,                             // The location
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
            }],
        });

        let image_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("image_bind_group"),
            layout: &bind_group_layout,
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
            ],
        });

        let cs_module = device.create_shader_module(&wgpu::include_spirv!("../shaders/viewer.comp.spv"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let image_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("image_pipeline"),
            layout: Some(&pipeline_layout),
            module: &cs_module,
            entry_point: "main",
            
        });

        Self {
            width,
            height,
            display_texture,
            display_sampler,
            image_pipeline,
            image_bind_group
        }
    }

    pub fn run(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut compute_encoder = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor{label: None});
        compute_encoder.set_pipeline(&self.image_pipeline);
        compute_encoder.set_bind_group(0, &self.image_bind_group, &[]);
        compute_encoder.dispatch(
            (self.width as f32 / 32_f32).ceil() as u32, 
            (self.width as f32 / 32_f32).ceil() as u32, 
            1);
        drop(compute_encoder);
    }

    pub fn texture(&self) -> &wgpu::Texture {
        &self.display_texture
    }

    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.display_sampler
    }
}