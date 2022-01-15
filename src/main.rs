#![allow(dead_code)]
use winit::{
    window::{Window,WindowBuilder},
    event::*,
    event_loop::{EventLoop, ControlFlow}
};
use winit_input_helper::WinitInputHelper;

mod game;

fn main() {
    let evloop = EventLoop::new();

    let window = WindowBuilder::new()
        .build(&evloop).unwrap();

    let mut input = WinitInputHelper::new();
    let mut state = pollster::block_on(game::State::new(&window));

    evloop.run(move |main_event, _, control_flow| {
        if input.update(&main_event) {
            // Update
            window.request_redraw();
        }

        match main_event {
            Event::WindowEvent {
                window_id,
                event: window_event
            } if window_id == window.id() => {
                match window_event {
                    WindowEvent::CloseRequested => { *control_flow = ControlFlow::Exit },
                    WindowEvent::Resized(new_size) => {
                        // Resize
                        state.resize(new_size);
                    },
                    _ => {}
                }
            },
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                // Render
                match state.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if lost
                    Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e),
                }
            },
            _ => {}
        }
    });
}
