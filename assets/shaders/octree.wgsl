#pragma once

#include "ray.wgsl"
#include "common.wgsl"

const SOLID_BIT = 1u;
const PARENT_BIT = 2u;
const SHADOW_BIT = 4u;

const EMPTY_MASK = 3u;

struct Node {
	flags: u32,
	data: u32,
}

fn node_is_solid(node: Node) -> bool {
	return (node.flags & SOLID_BIT) != 0u;
}

fn node_is_parent(node: Node) -> bool {
	return (node.flags & PARENT_BIT) != 0u;
}

fn node_is_shadow(node: Node) -> bool {
	return (node.flags & SHADOW_BIT) != 0u;
}

fn node_is_empty(node: Node) -> bool {
	return (node.flags & EMPTY_MASK) == 0u;
}

fn node_color(node: Node) -> vec4<f32> {
	return unpack4x8unorm(node.data);
}

fn node_pointer(node: Node) -> u32 {
	return node.data;
}

@group(2) @binding(0)
var voxels: texture_3d<u32>;

struct OctreeUniform {
	model: mat4x4<f32>,
	model_inv: mat4x4<f32>,
}

@group(2) @binding(1)
var<uniform> octree: OctreeUniform;

struct OctreeHit {
	hit: bool,
	node: Node,
	normal: vec3<f32>,
	position: vec3<f32>,
	step_count: u32,
}

fn octree_get_node(pointer: u32) -> Node {
	// indices are encoded as 12 bits for x, 12 bits for y, 8 bits for z
	let x = (pointer >>  0u) & 0xFFFu;
	let y = (pointer >> 12u) & 0xFFFu;
	let z = (pointer >> 24u) & 0xFFu;

	let data = textureLoad(voxels, vec3<u32>(x, y, z), 0);
	return Node(data.r, data.g);
}

fn octree_in_bounds(point: vec3<f32>) -> bool {
	return all(abs(point) < 1.0);
}

fn octree_project(origin: ptr<function, vec3<f32>>, direction: vec3<f32>) -> bool {
	if octree_in_bounds(*origin) { return true; }

	let tnear = (-1.0 - *origin) / direction;
	let tfar = (1.0 - *origin) / direction;

	let tmin = min(tnear, tfar);
	let tmax = max(tnear, tfar);

	let near = max(tmin.x, max(tmin.y, tmin.z));
	let far = min(tmax.x, min(tmax.y, tmax.z));

	if near > far || far < 0.0 { return false; }

	*origin += direction * near;

	return true;
}

fn octree_split(path: vec3<i32>, depth: u32) -> vec3<f32> {
	let scale = f32(1u << depth);
	return (vec3<f32>(path) + 0.5) / scale - 1.0;
}

fn octree_select_initial_child(pos: vec3<f32>) -> u32 {
	return u32(pos.x > 0.0) | (u32(pos.y > 0.0) << 1u) | (u32(pos.z > 0.0) << 2u);
}

fn octree_select_child(path: vec3<i32>, pos: vec3<f32>, depth: u32) -> u32 {
	let split = octree_split(path, depth);
	return u32(pos.x > split.x) | (u32(pos.y > split.y) << 1u) | (u32(pos.z > split.z) << 2u);
}

fn octree_add_child(path: vec3<i32>, child: u32) -> vec3<i32> {
	var path = path << 1u;

	if (child & 1u) != 0u { path.x |= 1; }
	if (child & 2u) != 0u { path.y |= 1; }
	if (child & 4u) != 0u { path.z |= 1; }

	return path;
}

fn octree_extract_child(path: vec3<i32>, depth: u32) -> u32 {
	return u32((path.x & (1 << depth)) != 0) 
		| (u32((path.y & (1 << depth)) != 0) << 1u) 
		| (u32((path.z & (1 << depth)) != 0) << 2u);
}

fn octree_ray_cast_normalized(ray: Ray, main_ray: bool) -> OctreeHit {
	var hit: OctreeHit;
	hit.hit = false;
	hit.step_count = 0u;

	// project ray origin to octree bounds, return miss if the ray does not intersect
	var position = ray.origin;
	if !octree_project(&position, ray.direction) { return hit; }
	hit.position = position;

	// compute normal of initial hit	
	hit.normal = vec3<f32>(sign(hit.position)) * vec3<f32>(abs(hit.position) > 1.0 - EPSILON);
	let dir = vec3<i32>(sign(ray.direction));	

	// handle root node
	let root = octree_get_node(0u);
	if node_is_empty(root) { return hit; }
	if node_is_solid(root) { 
		hit.hit = true; 
		hit.node = root;
		return hit; 
	}

	// initialize traversal
	var parent = node_pointer(root);
	var depth = 0u;
	var child = octree_select_initial_child(hit.position);
	var path = octree_add_child(vec3<i32>(0), child);
	var stack: array<u32, 32>;
	stack[0] = parent;

	for (var _i = 0u; _i < 256u; _i += 1u) {
		hit.step_count += 1u;

		let node = octree_get_node(parent + child);

		if node_is_parent(node) {
			parent = node_pointer(node);
			child = octree_select_child(path, hit.position, depth);
			path = octree_add_child(path, child);

			depth += 1u;
			stack[depth] = parent;
			continue;
		}

		// if node is solid, return hit
		if node_is_solid(node) && (main_ray || node_is_shadow(node)) {
			hit.hit = true;
			hit.node = node;
			break;
		}

		// otherwise, step along ray
		let old_path = path;
		let split = octree_split(path, depth);
		let bounds = split + vec3<f32>(dir) / f32(1 << depth + 1u);
		let t = (bounds - hit.position) / ray.direction;

		let tmin = min(t.x, min(t.y, t.z));
		if tmin == t.x { path.x += dir.x; hit.normal = vec3<f32>(f32(-dir.x), 0.0, 0.0); }
		if tmin == t.y { path.y += dir.y; hit.normal = vec3<f32>(0.0, f32(-dir.y), 0.0); }
		if tmin == t.z { path.z += dir.z; hit.normal = vec3<f32>(0.0, 0.0, f32(-dir.z)); }
	
		hit.position += ray.direction * tmin;

		// find first bit that differs between old and new path
		let path_diff = path ^ old_path;
		let diff = path_diff.x | path_diff.y | path_diff.z;
		let flip = u32(firstLeadingBit(diff));

		// if we've reached the end of the octree, return miss
		if flip > depth { break; }

		parent = stack[depth - flip];
		child = octree_extract_child(path, flip);

		for (var i = flip; i > 0u;) {
			let node = octree_get_node(parent + child);
			if !node_is_parent(node) {
				depth -= i;
				path >>= i;
				break;
			}

			i -= 1u;

			parent = node_pointer(node);
			child = octree_extract_child(path, i);
			stack[depth - i] = parent;
		}
	}

	return hit;
}

fn octree_ray_cast(ray: Ray, main_ray: bool) -> OctreeHit {
	let normalized_ray = ray_transform(ray, octree.model_inv);
	var hit = octree_ray_cast_normalized(normalized_ray, main_ray);

	if !hit.hit { return hit; }

	let position = octree.model * vec4<f32>(hit.position, 1.0);
	let normal = octree.model * vec4<f32>(hit.normal, 0.0);

	hit.position = position.xyz / position.w;
	hit.normal = normalize(normal.xyz);

	return hit;
}
