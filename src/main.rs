mod app;
mod fft;
mod frequency;
mod lattice;
mod phase;
mod renderer;
mod shape;
mod spectrum;
mod state;
mod ui;

use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let event_loop = EventLoop::new().expect("create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = app::App::new();
    event_loop.run_app(&mut app).expect("run app");
}
