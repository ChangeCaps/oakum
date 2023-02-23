#pragma once

#include "camera.wgsl"

var<private> COORDS: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
	vec2<f32>(-1.0, -1.0),
	vec2<f32>(-1.0, 1.0),
	vec2<f32>(1.0, 1.0),
	vec2<f32>(1.0, 1.0),
	vec2<f32>(1.0, -1.0),
	vec2<f32>(-1.0, -1.0),
);

struct VertexInput {
	@builtin(vertex_index)
	index: u32,
}

struct VertexOutput {
	@builtin(position)
	position: vec4<f32>,
	@location(0)
	clip: vec4<f32>,
	@location(1)
	uv: vec2<f32>,
}

@vertex
fn main(in: VertexInput) -> VertexOutput {
	let coords = COORDS[in.index];

	var out: VertexOutput;

	out.position = vec4<f32>(coords, 0.0, 1.0);
	out.clip = vec4<f32>(coords, 0.0, 1.0);
	out.uv = coords * vec2<f32>(0.5, -0.5) + 0.5;

	return out;
}
