use std::time::Instant;

use glam::{Mat4, Vec2, Vec3};
use winit::{
    event::{
        DeviceEvent, ElementState, Event, KeyboardInput, MouseButton,
        MouseScrollDelta::{LineDelta, PixelDelta},
        WindowEvent,
    },
    window::Window,
};

use crate::{
    generate::{GrassBlock, Sphere},
    input::{Key, Keyboard, Mouse},
    octree::{Branch, Octree},
    render::Renderer,
    world::World,
};

#[derive(Clone, Copy, Debug)]
pub struct UpdateContext<'a> {
    pub delta: f32,
    pub mouse: &'a Mouse,
    pub keyboard: &'a Keyboard,
}

pub struct App {
    pub world: World,
    pub renderer: Renderer,
    pub window: Window,
    pub mouse: Mouse,
    pub keyboard: Keyboard,
    pub last_frame: Instant,
    pub sphere: Octree,
}

impl App {
    pub unsafe fn new(window: Window) -> Self {
        let renderer = Renderer::new(&window).unwrap();
        let mut world = World::new();

        let grass = Octree::generate(&GrassBlock);
        let sphere = Octree::generate(&Sphere::new(32, 6));

        for x in -8..8 {
            for y in -8..8 {
                for z in -8..8 {
                    world
                        .octree
                        .union((x * 16 + 8, y * 16 + 8, z * 16 + 8, 10), 5, &grass);
                }
            }
        }

        Self {
            world,
            renderer,
            window,
            mouse: Mouse::default(),
            keyboard: Keyboard::default(),
            last_frame: Instant::now(),
            sphere,
        }
    }

    pub fn update(&mut self) -> anyhow::Result<()> {
        let now = Instant::now();
        let delta = now - self.last_frame;
        self.last_frame = now;

        let cx = UpdateContext {
            delta: delta.as_secs_f32(),
            mouse: &self.mouse,
            keyboard: &self.keyboard,
        };

        self.world.update(cx);

        if self.mouse.is_pressed(MouseButton::Right)
            || self.mouse.is_held(MouseButton::Right) && self.keyboard.is_held(Key::F)
        {
            let w = self.window.inner_size().width;
            let h = self.window.inner_size().height;
            let ray = self.world.camera.mouse_ray(w, h, self.mouse.position);

            let scale = Mat4::from_scale(Vec3::splat(10.0));
            if let Some(hit) = self.world.octree.raycast(scale, ray) {
                let mut branch = Branch::from_point(scale, hit.point, 10);
                branch.path += hit.normal;
                self.world.octree.difference(branch, 4, &self.sphere);
            }
        } else if self.mouse.is_pressed(MouseButton::Left)
            || self.mouse.is_held(MouseButton::Left) && self.keyboard.is_held(Key::F)
        {
            let w = self.window.inner_size().width;
            let h = self.window.inner_size().height;
            let ray = self.world.camera.mouse_ray(w, h, self.mouse.position);

            let scale = Mat4::from_scale(Vec3::splat(10.0));
            if let Some(hit) = self.world.octree.raycast(scale, ray) {
                let mut branch = Branch::from_point(scale, hit.point, 10);
                branch.path += hit.normal;
                self.world.octree.union(branch, 4, &self.sphere);
            }
        }

        Ok(())
    }

    pub fn post_update(&mut self) -> anyhow::Result<()> {
        self.mouse.update();
        self.keyboard.update();

        self.world.post_update();

        Ok(())
    }

    pub fn event(&mut self, event: &Event<()>) {
        match event {
            Event::DeviceEvent { event, .. } => match event {
                DeviceEvent::MouseMotion { delta } => {
                    self.mouse.delta += Vec2::new(delta.0 as f32, delta.1 as f32);
                }
                DeviceEvent::MouseWheel { delta } => {
                    self.mouse.scroll = match delta {
                        LineDelta(x, y) => Vec2::new(*x as f32, *y as f32),
                        PixelDelta(pos) => Vec2::new(pos.x as f32, pos.y as f32),
                    };
                }
                DeviceEvent::Key(KeyboardInput {
                    state,
                    virtual_keycode: Some(key),
                    ..
                }) => match state {
                    ElementState::Pressed => self.keyboard.press(*key),
                    ElementState::Released => self.keyboard.release(*key),
                },
                _ => {}
            },
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::MouseInput { state, button, .. } => match state {
                    ElementState::Pressed => self.mouse.press(*button),
                    ElementState::Released => self.mouse.release(*button),
                },
                WindowEvent::CursorMoved { position, .. } => {
                    self.mouse.position = Vec2::new(position.x as f32, position.y as f32);
                }
                _ => {}
            },
            _ => {}
        }
    }

    pub fn request_close(&self) -> bool {
        true
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn window_resized(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        self.update()?;
        self.renderer.render_frame(&self.world)?;
        self.post_update()?;

        Ok(())
    }
}
