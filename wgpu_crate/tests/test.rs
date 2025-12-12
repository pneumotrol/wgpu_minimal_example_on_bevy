use anyhow::Result;
use wgpu::{
    Backends, BufferDescriptor, BufferUsages, CommandEncoderDescriptor, DeviceDescriptor, Instance,
    InstanceDescriptor, MapMode, PollType, PowerPreference, RequestAdapterOptionsBase,
};
use wgpu_crate::{ComputeShader, StorageData, UniformData, Vec2f};

#[tokio::test]
async fn compute_success() -> Result<()> {
    const NUM_OF_DATA: usize = 144;

    // Prepare input data and ground truth
    let mut data = Vec::new();
    let mut ground_truth = Vec::new();
    for i in 0..NUM_OF_DATA {
        let angle = f32::to_radians(i as f32 * 90.0);
        data.push(StorageData {
            vector: Vec2f {
                x: angle.cos(),
                y: angle.sin(),
            },
        });

        let rotated_angle = angle + f32::to_radians(90.0);
        ground_truth.push(StorageData {
            vector: Vec2f {
                x: rotated_angle.cos(),
                y: rotated_angle.sin(),
            },
        });
    }

    // Initialize WGPU
    let (device, queue) = Instance::new(&InstanceDescriptor {
        backends: Backends::PRIMARY,
        ..Default::default()
    })
    .request_adapter(&RequestAdapterOptionsBase {
        power_preference: PowerPreference::HighPerformance,
        ..Default::default()
    })
    .await?
    .request_device(&DeviceDescriptor::default())
    .await?;

    let compute_shader = ComputeShader::new(&device)?;

    // Upload data to GPU
    queue.write_buffer(
        compute_shader.storage_buffer(),
        0,
        bytemuck::cast_slice(&data),
    );
    queue.write_buffer(
        compute_shader.uniform_buffer(),
        0,
        bytemuck::bytes_of(&UniformData { rotate_deg: 90.0 }),
    );

    // Encode and submit compute pass
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());
    compute_shader.encode_compute_pass(&mut encoder, NUM_OF_DATA as u32);
    queue.submit(Some(encoder.finish()));

    // Encode and submit data copy pass
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());
    let result_buffer = device.create_buffer(&BufferDescriptor {
        label: Some("result_buffer"),
        size: (std::mem::size_of::<StorageData>() * NUM_OF_DATA) as u64,
        usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    compute_shader.encode_data_copy(&mut encoder, &result_buffer)?;
    queue.submit(Some(encoder.finish()));

    // Download data from GPU
    let (tx, rx) = crossbeam_channel::bounded(1);
    result_buffer
        .slice(..)
        .map_async(MapMode::Read, move |v| tx.send(v).unwrap());
    device.poll(PollType::Wait)?;
    rx.recv()??;

    let result_data: Vec<StorageData> =
        bytemuck::cast_slice(&result_buffer.slice(..).get_mapped_range()).to_vec();
    result_buffer.unmap();

    // Verify results
    for (i, (result, truth)) in result_data.iter().zip(ground_truth.iter()).enumerate() {
        let diff_x = (result.vector.x - truth.vector.x).abs();
        let diff_y = (result.vector.y - truth.vector.y).abs();
        assert!(diff_x < 1e-5, "Mismatch at index {}: x diff {}", i, diff_x);
        assert!(diff_y < 1e-5, "Mismatch at index {}: y diff {}", i, diff_y);
    }

    Ok(())
}
