#pragma once

#include "ray.wgsl"

struct Camera {
	view: mat4x4<f32>,
	proj: mat4x4<f32>,
	view_proj: mat4x4<f32>,
	view_inv: mat4x4<f32>,
	proj_inv: mat4x4<f32>,
	view_proj_inv: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

fn world_to_view(world: vec3<f32>) -> vec4<f32> {
	return camera.view * vec4<f32>(world, 1.0);
}

fn view_to_world(view: vec4<f32>) -> vec3<f32> {
	let world = camera.view_inv * view;
	return world.xyz / world.w;
}

fn view_to_clip(view: vec4<f32>) -> vec4<f32> {
	return camera.proj * view;
}

fn clip_to_view(clip: vec4<f32>) -> vec4<f32> {
	let view = camera.proj_inv * clip;
	return view / view.w;
}

fn world_to_clip(world: vec3<f32>) -> vec4<f32> {
	return camera.view_proj * vec4<f32>(world, 1.0);
}

fn clip_to_world(clip: vec4<f32>) -> vec3<f32> {
	let world = camera.view_proj_inv * clip;
	return world.xyz / world.w;
}

fn camera_ray(coord: vec2<f32>) -> Ray {
	let near = clip_to_world(vec4<f32>(coord, 0.0, 1.0));
	let far = clip_to_world(vec4<f32>(coord, 1.0, 1.0));

	return Ray(near, normalize(far - near));
}
