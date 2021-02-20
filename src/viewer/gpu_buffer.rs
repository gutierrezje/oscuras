use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

pub struct GPUBufferDescription<'a, T> where T: Pod + Zeroable {
    pub binding_type: wgpu::BufferBindingType,
    pub contents: Option<&'a [T]>,
    pub element_count: u32,
    pub element_size: usize,
    pub usage: wgpu::BufferUsage,
}

pub struct GPUBuffer {
    binding_type: wgpu::BufferBindingType,
    element_count: u32,
    handle: wgpu::Buffer,
    size: wgpu::BufferAddress,
    usage: wgpu::BufferUsage,
}

impl GPUBuffer {
    pub fn new<T: Pod + Zeroable>(device: &wgpu::Device, buffer_desc: GPUBufferDescription<T>)-> Self {
        let binding_type = buffer_desc.binding_type;
        let element_count = buffer_desc.element_count;
        let size = (element_count as usize * buffer_desc.element_size) as wgpu::BufferAddress;
        let usage = buffer_desc.usage;
        let handle;
        match buffer_desc.contents {
            Some(contents) => {
                handle = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(contents),
                    usage,
                });
                
            },
            None =>  {
                handle = device.create_buffer(&wgpu::BufferDescriptor {
                    label: None,
                    mapped_at_creation: false,
                    size,
                    usage,
                });
            }
        }
        Self {binding_type, element_count, handle, size, usage}
    }

    pub fn as_bgl_entry(&self, binding: u32, visibility: wgpu::ShaderStage, _read_only: bool) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: match self.binding_type {
                    wgpu::BufferBindingType::Storage{read_only} => {
                        match read_only == _read_only {
                            false => wgpu::BufferBindingType::Storage {read_only: _read_only},
                            true => self.binding_type
                        }
                    },
                    _ => self.binding_type,
                },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }
    }

    pub fn as_bg_entry(&self, binding: u32) -> wgpu::BindGroupEntry {
        wgpu::BindGroupEntry {
            binding,
            resource: self.handle.as_entire_binding(),
        }
    }
}