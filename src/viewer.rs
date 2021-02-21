use data_types::VERTICES;
use wgpu::util::DeviceExt;
use winit::{event::*, window::Window};

mod camera;
mod data_types;
mod gpu_buffer;
mod pathtracer;
mod scene;

use camera::Camera;
use data_types::Sphere;
use gpu_buffer::{GPUBuffer, GPUBufferDescription};
use pathtracer::Pathtracer;
use scene::Scene;

pub struct Viewer {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    size: winit::dpi::PhysicalSize<u32>,
    viewer_pipeline: wgpu::RenderPipeline,
    viewer_bg: wgpu::BindGroup,
    vertex_buffer: GPUBuffer,
    num_vertices: u32,
    camera: Camera,
    scene: Scene,
    pathtracer: Pathtracer,
}

impl Viewer {
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        let camera = Camera::new(&size);
        let mut geoms = Vec::<Sphere>::new();
        geoms.push(Sphere {
            center: [0.0, 0.0, 1.0],
            radius: 0.5f32,
        });
        let scene = Scene { geometry: geoms };

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12+ Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("device"),
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .unwrap();

        let swapchain_format = adapter.get_swap_chain_preferred_format(&surface);

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let pathtracer = Pathtracer::new(&device, &camera, &scene);

        // Set up the vertex buffer for our quad
        let num_vertices = data_types::VERTICES.len() as u32;
        let vert_buf_desc = GPUBufferDescription::<data_types::Vertex> {
            contents: Some(data_types::VERTICES),
            element_count: num_vertices,
            element_size: std::mem::size_of::<data_types::Vertex>(),
            usage: wgpu::BufferUsage::VERTEX,
        };
        let vertex_buffer = GPUBuffer::new(&device, vert_buf_desc);

        // Shader setup
        let vs_module =
            device.create_shader_module(&wgpu::include_spirv!("shaders/viewer.vert.spv"));
        let fs_module =
            device.create_shader_module(&wgpu::include_spirv!("shaders/viewer.frag.spv"));

        let viewer_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0, // The location
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler {
                        filtering: true,
                        comparison: false,
                    },
                    count: None,
                },
            ],
        });

        let viewer_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("display_bind_group"),
            layout: &viewer_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &pathtracer
                            .texture()
                            .create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&pathtracer.sampler()),
                },
            ],
        });

        let viewer_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&viewer_bgl],
            push_constant_ranges: &[],
        });

        let viewer_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&viewer_pl),
            vertex: wgpu::VertexState {
                module: &vs_module,
                entry_point: "main",
                buffers: &[data_types::Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &fs_module,
                entry_point: "main",
                targets: &[swapchain_format.into()],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
        });

        Self {
            surface,
            device,
            queue,
            sc_desc,
            swap_chain,
            size,
            viewer_pipeline,
            vertex_buffer,
            num_vertices,
            viewer_bg,
            camera,
            scene,
            pathtracer,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }

    pub fn update(&mut self) {}

    pub fn render(&mut self) -> Result<(), wgpu::SwapChainError> {
        let frame = self.swap_chain.get_current_frame()?.output;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.pathtracer.run(&mut encoder);

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: &frame.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&self.viewer_pipeline);
        render_pass.set_bind_group(0, &self.viewer_bg, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.contents());
        render_pass.draw(0..self.num_vertices, 0..1);
        drop(render_pass);

        // submite will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }

    pub fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.size
    }
}
