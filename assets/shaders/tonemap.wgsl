#pragma once

@group(0) @binding(0)
var hdr_texture: texture_2d_array<f32>;

struct FragmentInput {
	@location(0) 
	clip: vec4<f32>,
	@location(1) 
	uv: vec2<f32>,
};

fn tonemap_aces(rgb: vec3<f32>) -> vec3<f32> {
	let a = 2.51;
	let b = 0.03;
	let c = 2.43;
	let d = 0.59;
	let e = 0.14;
	return (rgb * (a * rgb + b)) / (rgb * (c * rgb + d) + e);
}

@fragment
fn main(in: FragmentInput) -> @location(0) vec4<f32> {
	let dimensions = textureDimensions(hdr_texture);
	let index = vec2<i32>(in.uv * vec2<f32>(dimensions));

	var color: vec4<f32> = vec4<f32>(0.0);

	let samples = textureNumLayers(hdr_texture);
	for (var i = 0; i < samples; i += 1) {
		color = color + textureLoad(hdr_texture, index, i, 0);
	}

	color = color / f32(samples);

	return vec4<f32>(color.rgb, 1.0);
}
