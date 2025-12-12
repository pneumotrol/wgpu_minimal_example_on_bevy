struct StorageData {
    vector: vec2f,
}

struct UniformData {
    rotate_deg: f32,
}

@group(0) @binding(0) var<storage, read_write> storage_buffer: array<StorageData>;
@group(0) @binding(1) var<uniform> uniform_buffer: UniformData;

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index: u32 = global_id.x;
    if index >= arrayLength(&storage_buffer) {
        return;
    }

    let rad: f32 = radians(uniform_buffer.rotate_deg);
    let rot: mat2x2f = mat2x2f(vec2f(cos(rad), sin(rad)), vec2f(-sin(rad), cos(rad)));

    storage_buffer[index].vector = rot * storage_buffer[index].vector;
}
