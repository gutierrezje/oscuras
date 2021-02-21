use bytemuck::{Pod, Zeroable};
use wgpu::{util::DeviceExt, BufferSlice};

pub struct GPUBufferDescription<'a, T>
where
    T: Pod + Zeroable,
{
    pub contents: Option<&'a [T]>,
    pub element_count: u32,
    pub element_size: usize,
    pub usage: wgpu::BufferUsage,
}

pub struct GPUBuffer {
    element_count: u32,
    handle: wgpu::Buffer,
    size: wgpu::BufferAddress,
    usage: wgpu::BufferUsage,
}

impl GPUBuffer {
    pub fn new<T: Pod + Zeroable>(
        device: &wgpu::Device,
        buffer_desc: GPUBufferDescription<T>,
    ) -> Self {
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
            }
            None => {
                handle = device.create_buffer(&wgpu::BufferDescriptor {
                    label: None,
                    mapped_at_creation: false,
                    size,
                    usage,
                });
            }
        }
        Self {
            element_count,
            handle,
            size,
            usage,
        }
    }

    pub fn contents(&self) -> BufferSlice {
        self.handle.slice(..)
    }

    pub fn as_bgl_entry(
        &self,
        binding: u32,
        visibility: wgpu::ShaderStage,
        _read_only: bool,
    ) -> wgpu::BindGroupLayoutEntry {
        let ty;
        if self.usage & wgpu::BufferUsage::STORAGE == wgpu::BufferUsage::STORAGE {
            ty = wgpu::BufferBindingType::Storage {
                read_only: _read_only,
            };
        } else if self.usage & wgpu::BufferUsage::UNIFORM == wgpu::BufferUsage::UNIFORM {
            ty = wgpu::BufferBindingType::Uniform;
        } else {
            panic!("Trying to make a Bind Group Entry with an unsupported buffer type");
        }
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty,
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
