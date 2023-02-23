#pragma once

struct Ray {
	origin: vec3<f32>,
	direction: vec3<f32>,
}

fn ray_transform(ray: Ray, matrix: mat4x4<f32>) -> Ray {
	let origin = matrix * vec4<f32>(ray.origin, 1.0);
	let direction = matrix * vec4<f32>(ray.direction, 0.0);

	return Ray(origin.xyz / origin.w, direction.xyz);
}
