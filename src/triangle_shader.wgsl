struct VertexInput {
	@location(0) position: vec2<f32>,
	@location(1) uv_coordinates: vec2<f32>,
};

struct InstanceInput {
	@location(2) position: vec2<f32>,
}

struct VertexOutput {
	@builtin(position) clip_position: vec4<f32>,
	@location(0) uv_coordinates: vec2<f32>,
};

@vertex
fn vs_main(
	model: VertexInput,
) -> VertexOutput {
	var out: VertexOutput;
	let position = model.position;
	out.clip_position = vec4<f32>(position, 0.0, 1.0);
	out.uv_coordinates = model.uv_coordinates;
	return out;
}

@group(0) @binding(0)
var<uniform> mode: u32;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	if mode == 0 {
		return vec4<f32>(0.216, 0.022, 0.022, 1.0);
	} else if mode == 1 {
		if pow(in.uv_coordinates[0], 2) <= in.uv_coordinates[1] {
			return vec4<f32>(0.216, 0.022, 0.022, 1.0);
		} else {
			return vec4<f32>(0, 0, 0, 0);
		}
	} else if mode == 2 {
		if pow(in.uv_coordinates[0], 2) >= in.uv_coordinates[1] {
			return vec4<f32>(0.216, 0.022, 0.022, 1.0);
		} else {
			return vec4<f32>(0, 0, 0, 0);
		}
	} else {
		return vec4<f32>(1, 0, 0, 1.0);
	}
}