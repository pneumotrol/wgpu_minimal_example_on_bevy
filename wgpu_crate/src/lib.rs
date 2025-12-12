use anyhow::{Result, anyhow};
use bytemuck::{Pod, Zeroable};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferDescriptor, BufferUsages,
    CommandEncoder, ComputePassDescriptor, ComputePipeline, ComputePipelineDescriptor, Device,
    PipelineLayoutDescriptor, ShaderModuleDescriptor, ShaderSource, ShaderStages,
};

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Pod, Zeroable)]
pub struct Vec2f {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct StorageData {
    pub vector: Vec2f,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct UniformData {
    pub rotate_deg: f32,
}

pub struct ComputeShader {
    bind_group: BindGroup,
    storage_buffer: Buffer,
    uniform_buffer: Buffer,
    pipeline: ComputePipeline,
}

impl ComputeShader {
    const MAX_DATA_SIZE: usize = 1024;

    pub fn new(device: &Device) -> Result<Self> {
        // Bind group layout
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("compute_bind_group_layout"),
            entries: &[
                // Binding 0: Storage buffer
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Binding 1: Uniform buffer
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Buffer
        let size = std::mem::size_of::<StorageData>()
            .checked_mul(Self::MAX_DATA_SIZE)
            .ok_or_else(|| anyhow!("Buffer size overflow"))?
            .try_into()?;
        let storage_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("storage_buffer"),
            size,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let size = std::mem::size_of::<UniformData>().try_into()?;
        let uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("uniform_buffer"),
            size,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Bind group
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("bind_group"),
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: storage_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        // Pipeline
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("shader"),
            source: ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        Ok(Self {
            bind_group,
            storage_buffer,
            uniform_buffer,
            pipeline,
        })
    }

    pub fn encode_compute_pass(&self, encoder: &mut CommandEncoder, num_of_data: u32) {
        {
            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor::default());
            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &self.bind_group, &[]);
            compute_pass.dispatch_workgroups(num_of_data, 1, 1);
        }
    }

    pub fn encode_data_copy(&self, encoder: &mut CommandEncoder, buffer: &Buffer) -> Result<()> {
        if self.storage_buffer.size() < buffer.size() {
            return Err(anyhow!("Destination buffer is smaller than source buffer"));
        }

        encoder.copy_buffer_to_buffer(&self.storage_buffer, 0, buffer, 0, buffer.size());

        Ok(())
    }

    pub fn storage_buffer(&self) -> &Buffer {
        &self.storage_buffer
    }

    pub fn uniform_buffer(&self) -> &Buffer {
        &self.uniform_buffer
    }
}
