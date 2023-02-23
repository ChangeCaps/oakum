#![allow(dead_code)]

use app::App;
use clap::Parser;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod app;
mod generate;
mod input;
mod octree;
mod ray;
mod render;
mod world;

#[derive(Parser)]
pub struct Args {
    #[clap(short, long, default_value = "info")]
    pub log_level: log::LevelFilter,
}

impl Args {
    pub fn init_logger(&self) {
        env_logger::builder()
            .filter_level(self.log_level)
            .filter_module("wgpu", log::LevelFilter::Warn)
            .filter_module("winit", log::LevelFilter::Warn)
            .filter_module("naga", log::LevelFilter::Warn)
            .init();
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    args.init_logger();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Oakum")
        .build(&event_loop)
        .unwrap();

    let mut app = unsafe { App::new(window) };

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match &event {
            Event::RedrawRequested(_) => {
                if let Err(e) = app.render() {
                    eprintln!("Error: {}", e);
                }
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    if app.request_close() {
                        *control_flow = ControlFlow::Exit;
                    }
                }
                &WindowEvent::Resized(size)
                | WindowEvent::ScaleFactorChanged {
                    new_inner_size: &mut size,
                    ..
                } => {
                    app.window_resized(size.width, size.height);
                }
                _ => {}
            },
            Event::RedrawEventsCleared => {
                app.request_redraw();
            }
            _ => {}
        }

        app.event(&event);
    });
}
