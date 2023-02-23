#pragma once

#include "octree.wgsl"
#include "ray.wgsl"
#include "common.wgsl"
#include "tonemap.wgsl"
#include "camera.wgsl"

struct FragmentInput {
	@location(0)
	clip: vec3<f32>,
}

@fragment
fn main(in: FragmentInput) -> @location(0) vec4<f32> {
	let ray = camera_ray(in.clip.xy);

	let hit = octree_intersect(ray);
	if !hit.hit { return vec4<f32>(0.0); }

	let sun_dir = normalize(vec3<f32>(0.9, 1.0, -0.8));
	let sky_dir = normalize(vec3<f32>(0.0, 1.0, 0.0));
	let bou_dir = normalize(vec3<f32>(0.0, -1.0, 0.0));

	let sun_dif = max(dot(sun_dir, hit.normal), 0.0);
	let sky_dif = max(dot(sky_dir, hit.normal), 0.2) + 0.2;
	let bou_dif = max(dot(bou_dir, hit.normal), 0.2) + 0.2;

	let shadow_ray = Ray(hit.position + hit.normal * EPSILON, sun_dir);
	let shadow_hit = octree_intersect(shadow_ray);
	let shadow = f32(!shadow_hit.hit);

	let sun_light = vec3<f32>(1.0, 0.9, 0.8) * sun_dif * shadow;
	let sky_light = vec3<f32>(0.2, 0.3, 0.4) * sky_dif;

	let ambient = vec3<f32>(0.1, 0.1, 0.1);

	let light = sun_light + sky_light + ambient;

	let color = node_color(hit.node).rgb * light;
	return vec4<f32>(tonemap_aces(color), 1.0);
}
