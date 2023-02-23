#pragma once

#include "octree.wgsl"
#include "ray.wgsl"
#include "common.wgsl"
#include "camera.wgsl"
#include "poisson.wgsl"

struct Uniforms {
	taa_sample: u32,
	dimensions: vec2<u32>,
}

@group(0) @binding(1)
var<uniform> uniforms: Uniforms;

fn sample(clip: vec2<f32>) -> vec4<f32> {
	let ray = camera_ray(clip);

	let hit = octree_ray_cast(ray, true);
	if !hit.hit { return vec4<f32>(0.48, 0.84, 0.83, 1.0); }

	let sun_dir = normalize(vec3<f32>(0.9, 1.0, -0.8));
	var sun_dif = abs(dot(sun_dir, normalize(hit.normal))) * 0.35 + 0.65;	

	let shadow_ray = Ray(hit.position + hit.normal * EPSILON, sun_dir);
	let shadow_hit = octree_ray_cast(shadow_ray, false);
	let shadow = f32(!shadow_hit.hit) * 0.3 + sun_dif * 0.7;

	let color = node_color(hit.node).rgb * shadow;
	return vec4<f32>(color, 1.0);
}

struct FragmentInput {
	@location(0)
	clip: vec3<f32>,
	@location(1)
	uv: vec2<f32>,
}

@fragment
fn main(in: FragmentInput) -> @location(0) vec4<f32> {
	let offset = POISSON_DISK[uniforms.taa_sample] / vec2<f32>(uniforms.dimensions);
	return sample(in.clip.xy * vec2<f32>(1.0, -1.0) + offset);
}
