struct VertexInput {
	@location(0) position: vec2<f32>,
};

struct InstanceInput {
	@location(1) position: vec2<f32>,
	@location(2) size: vec2<f32>,
	@location(3) colour: vec3<f32>,
}

struct VertexOutput {
	@builtin(position) clip_position: vec4<f32>,
	@location(0) uv_coordinate: vec2<f32>,
	@location(1) colour: vec3<f32>,
};

@vertex
fn vs_main(
	model: VertexInput,
	instance: InstanceInput,
) -> VertexOutput {
	var out: VertexOutput;
	out.uv_coordinate = model.position * 2 - 1;
	let position = (model.position * instance.size) + instance.position;
	out.clip_position = vec4<f32>(position, 0.0, 1.0);
	out.colour = instance.colour;
	return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	if pow(in.uv_coordinate[0], 2) + pow(in.uv_coordinate[1], 2) < 1 {
		return vec4<f32>(in.colour, 1.0);
	} else {
		return vec4<f32>(0.0, 0.0, 0.0, 0.0);
	}
}