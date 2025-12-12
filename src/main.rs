use bevy::{
    color::palettes::css::{GRAY, YELLOW},
    dev_tools::fps_overlay::FpsOverlayPlugin,
    prelude::*,
    render::{
        render_resource::{
            BufferDescriptor, BufferUsages, CommandEncoderDescriptor, MapMode, PollType,
        },
        renderer::{RenderDevice, RenderQueue},
    },
};
use rand::Rng;
use wgpu_crate::{ComputeShader, StorageData, UniformData, Vec2f};

#[derive(Resource, Deref, DerefMut)]
struct ComputeShaderResource(ComputeShader);

const NUM_OF_DATA: usize = 144;

fn main() {
    const FRAMERATE: f64 = 60.0;

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(FpsOverlayPlugin::default())
        .insert_resource(Time::<Fixed>::from_hz(FRAMERATE))
        .add_systems(Startup, startup)
        .add_systems(FixedUpdate, compute)
        .add_systems(Update, (update_gizmos, update_angle, update_gizmos))
        .run();
}

fn startup(mut commands: Commands, device: Res<RenderDevice>, queue: Res<RenderQueue>) {
    // Add a 2D camera
    commands.spawn(Camera2d);

    // Create compute shader
    if let Ok(compute_shader) = ComputeShader::new(device.wgpu_device()) {
        // Initialize storage buffer with some data
        let mut data = Vec::new();
        let mut rng = rand::rng();
        for _ in 0..NUM_OF_DATA {
            let angle = f32::to_radians(rng.random_range(0.0..360.0));
            data.push(StorageData {
                vector: Vec2f {
                    x: angle.cos(),
                    y: angle.sin(),
                },
            });
        }
        queue.write_buffer(
            compute_shader.storage_buffer(),
            0,
            bytemuck::cast_slice(&data),
        );

        // Initialize uniform buffer with rotation angle
        let data = UniformData { rotate_deg: 0.0 };
        queue.write_buffer(
            compute_shader.uniform_buffer(),
            0,
            bytemuck::bytes_of(&data),
        );

        // Store compute shader as a resource
        commands.insert_resource(ComputeShaderResource(compute_shader));

        info!("Compute shader created successfully");
    } else {
        error!("Failed to create compute shader");
    }
}

fn compute(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    compute_shader: Res<ComputeShaderResource>,
) {
    let device = device.wgpu_device();
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());

    compute_shader.encode_compute_pass(&mut encoder, NUM_OF_DATA as u32);
    queue.submit(Some(encoder.finish()));
}

fn update_angle(queue: Res<RenderQueue>, compute_shader: Res<ComputeShaderResource>) {
    let data = UniformData { rotate_deg: 2.0 };
    queue.write_buffer(
        compute_shader.uniform_buffer(),
        0,
        bytemuck::bytes_of(&data),
    );
}

fn update_gizmos(
    compute_shader: Res<ComputeShaderResource>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut gizmos: Gizmos,
) {
    let device = device.wgpu_device();
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());

    // Copy storage buffer (GPU) to a new buffer (GPU, CPU readable)
    let buffer = device.create_buffer(&BufferDescriptor {
        label: Some("result_buffer"),
        size: (std::mem::size_of::<StorageData>() * NUM_OF_DATA) as u64,
        usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    compute_shader
        .encode_data_copy(&mut encoder, &buffer)
        .unwrap();
    queue.submit(Some(encoder.finish()));

    // Copy the new buffer (GPU) to a RAM (CPU)
    let (tx, rx) = crossbeam_channel::bounded(1);
    buffer
        .slice(..)
        .map_async(MapMode::Read, move |result| tx.send(result).unwrap());

    device.poll(PollType::Wait).unwrap();
    if rx.recv().is_err() {
        error!("Failed to map buffer for reading");
        return;
    }

    let result: Vec<StorageData> =
        bytemuck::cast_slice(&buffer.slice(..).get_mapped_range()).to_vec();
    buffer.unmap();

    // Draw gizmos
    let scale = 50.0;
    gizmos.grid_2d(
        Isometry2d::IDENTITY,
        (18, 11).into(),
        (scale, scale).into(),
        GRAY,
    );
    for (i, v) in result.iter().enumerate() {
        let col = 16;
        let x = (i % col) as f32 + 0.5;
        let y = (i / col) as f32 + 0.5;
        let base = Vec2::new(x, y) - Vec2::new(col as f32, (NUM_OF_DATA / col) as f32) * 0.5;

        gizmos.arrow_2d(
            base * scale,
            (base + Vec2::new(v.vector.x, v.vector.y) * 0.5) * scale,
            YELLOW,
        );
    }
}
