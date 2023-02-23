use std::{f32::consts::FRAC_2_PI, mem};

use bytemuck::{Pod, Zeroable};
use glam::{EulerRot, Mat4, Quat, Vec2, Vec3, Vec4, Vec4Swizzles};
use winit::event::MouseButton;

use crate::{app::UpdateContext, input::Key, ray::Ray};

#[derive(Clone, Debug)]
pub struct Camera {
    pub position: Vec3,
    pub distance: f32,
    pub rotation: Vec3,
    pub fov: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            distance: 4.0,
            rotation: Vec3::new(-FRAC_2_PI, FRAC_2_PI, 0.0),
            fov: 60.0,
        }
    }
}

impl Camera {
    pub fn new(position: Vec3, distance: f32, fov: f32) -> Self {
        Self {
            position,
            distance,
            rotation: Vec3::new(-FRAC_2_PI, FRAC_2_PI, 0.0),
            fov,
        }
    }

    pub fn rotation_quat(&self) -> Quat {
        Quat::from_euler(
            EulerRot::YXZ,
            self.rotation.y,
            self.rotation.x,
            self.rotation.z,
        )
    }

    pub fn update(&mut self, cx: UpdateContext) {
        if cx.mouse.is_held(MouseButton::Middle) {
            self.rotation.y -= cx.mouse.delta.x * 0.003;
            self.rotation.x -= cx.mouse.delta.y * 0.003;
        }

        let mut right = self.rotation_quat() * Vec3::X;
        let mut forward = self.rotation_quat() * Vec3::NEG_Z;

        right.y = 0.0;
        forward.y = 0.0;

        let mut movement = Vec3::ZERO;

        if cx.keyboard.is_held(Key::W) {
            movement += forward;
        }

        if cx.keyboard.is_held(Key::S) {
            movement -= forward;
        }

        if cx.keyboard.is_held(Key::A) {
            movement -= right;
        }

        if cx.keyboard.is_held(Key::D) {
            movement += right;
        }

        if cx.keyboard.is_held(Key::Space) {
            movement += Vec3::Y;
        }

        if cx.keyboard.is_held(Key::LShift) {
            movement -= Vec3::Y;
        }

        self.position += movement.normalize_or_zero() * cx.delta;
        self.distance += cx.mouse.scroll.y * 0.001;
    }

    pub fn view(&self) -> Mat4 {
        let rotation = self.rotation_quat();
        let position = rotation * Vec3::new(0.0, 0.0, self.distance) + self.position;
        Mat4::from_rotation_translation(rotation, position)
    }

    pub fn proj(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh(self.fov.to_radians(), aspect, 0.001, 1000.0)
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        self.proj(aspect) * self.view().inverse()
    }

    pub fn mouse_ray(&self, width: u32, height: u32, position: Vec2) -> Ray {
        let inv = self.view_proj(width as f32 / height as f32).inverse();

        let x = position.x / width as f32 * 2.0 - 1.0;
        let y = position.y / height as f32 * -2.0 + 1.0;

        let near = inv * Vec4::new(x, y, 0.0, 1.0);
        let far = inv * Vec4::new(x, y, 1.0, 1.0);

        let origin = near.xyz() / near.w;
        let direction = (far.xyz() / far.w - origin).normalize_or_zero();

        Ray::new(origin, direction)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct CameraData {
    pub view: Mat4,
    pub proj: Mat4,
    pub view_proj: Mat4,
    pub view_inv: Mat4,
    pub proj_inv: Mat4,
    pub view_proj_inv: Mat4,
}

pub struct DrawCamera {
    pub buffer: wgpu::Buffer,
}

impl DrawCamera {
    pub fn new(device: &wgpu::Device) -> anyhow::Result<Self> {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera Buffer"),
            size: mem::size_of::<CameraData>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self { buffer })
    }

    pub fn write(&self, queue: &wgpu::Queue, camera: &Camera, aspect: f32) {
        let view = camera.view();
        let proj = camera.proj(aspect);
        let view_proj = camera.view_proj(aspect);
        let view_inv = view.inverse();
        let proj_inv = proj.inverse();
        let view_proj_inv = view_proj.inverse();

        let data = CameraData {
            view,
            proj,
            view_proj,
            view_inv,
            proj_inv,
            view_proj_inv,
        };

        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(&data));
    }
}
