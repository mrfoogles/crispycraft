struct VertexInput {
    [[location(0)]] position: vec3<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] model_position: vec3<f32>;
};

struct Camera {
    transform: mat4x4<f32>;
};
[[group(0), binding(0)]]
var<uniform> camera: Camera;

[[stage(vertex)]]
fn vs_main(
    model: VertexInput
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.transform * vec4<f32>(model.position, 1.0);
    out.model_position = model.position;
    return out;
}

// Fragment shader

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(
        sin(in.model_position.x),
        sin(in.model_position.y),
        sin(in.model_position.z),
        1.
    );
}