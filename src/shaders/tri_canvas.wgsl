// Vertex shader

struct VertexInput {
    @location(0) position: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
	
    out.color = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    out.clip_position = vec4<f32>((model.position.x / 1024.0 - 0.5) * 2.0, -(model.position.y / 768.0 - 0.5) * 2.0, 0.0, 1.0);
    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color);
}
