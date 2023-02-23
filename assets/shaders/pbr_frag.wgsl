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

struct FragmentInput {
	#include "fullscreen_input.wgsl"
}

struct FragmentOutput {
	@builtin(frag_depth)
	depth: f32,
	@location(0)
	color: vec4<f32>,	
}

fn sample(clip: vec2<f32>) -> FragmentOutput {
	let ray = camera_ray(clip);

	let hit = octree_ray_cast(ray, true);
	if !hit.hit { discard; }

	let sun_dir = normalize(vec3<f32>(0.9, 1.0, -0.8));
	var sun_dif = abs(dot(sun_dir, normalize(hit.normal))) * 0.5 + 0.5;

	let shadow_ray = Ray(hit.position + hit.normal * EPSILON, sun_dir);
	let shadow_hit = octree_ray_cast(shadow_ray, false);
	let shadow = f32(!shadow_hit.hit) * 0.2 + sun_dif * 0.8;

	let clip = world_to_clip(hit.position);
	let color = node_color(hit.node).rgb * shadow;

	var out: FragmentOutput;
	out.depth = clip.z / clip.w;
	out.color = vec4<f32>(color, 1.0);

	return out;
}

@fragment
fn main(in: FragmentInput) -> FragmentOutput {
	let offset = POISSON_DISK[uniforms.taa_sample] / vec2<f32>(uniforms.dimensions);
	return sample(in.clip.xy + offset);
}
