#pragma once

#include "octree.wgsl"
#include "ray.wgsl"
#include "common.wgsl"
#include "camera.wgsl"
#include "poisson.wgsl"

@group(0) @binding(1)
var render_target: texture_storage_2d_array<rgba16float, write>;

@group(0) @binding(2)
var<uniform> taa_sample: i32;

fn sample(clip: vec2<f32>) -> vec4<f32> {
	let ray = camera_ray(clip);

	let hit = octree_ray_cast(ray, true);

	//if true { return vec4<f32>(f32(hit.step_count) / 64.0, 0.0, 0.0, 0.0); }

	if !hit.hit { return vec4<f32>(0.48, 0.84, 0.83, 1.0); }

	let sun_dir = normalize(vec3<f32>(0.9, 1.0, -0.8));
	var sun_dif = dot(sun_dir, normalize(hit.normal)) * 0.35 + 0.7;

	if !node_is_shadow(hit.node) {
		sun_dif = abs(dot(sun_dir, normalize(hit.normal))) * 0.6 + 0.6;
	}

	let shadow_ray = Ray(hit.position + hit.normal * EPSILON, sun_dir);
	let shadow_hit = octree_ray_cast(shadow_ray, false);
	let shadow = f32(!shadow_hit.hit) * 0.3 + sun_dif;

	let color = node_color(hit.node).rgb * shadow;
	return vec4<f32>(color, 1.0);
}

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {	
	let dimensions = textureDimensions(render_target);

	if any(global_id.xy >= vec2<u32>(dimensions.xy)) { return; }

	let offset = POISSON_DISK[taa_sample] / vec2<f32>(dimensions.xy);

	let uv = vec2<f32>(global_id.xy) / vec2<f32>(dimensions.xy);
	let color = sample(uv * 2.0 - 1.0 + offset);

	textureStore(render_target, global_id.xy, taa_sample, color);
}
