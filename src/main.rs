use winit::{
    window::{Window,WindowBuilder},
    event::*,
    event_loop::{EventLoop, ControlFlow}
};
use winit_input_helper::WinitInputHelper;

fn main() {
    let evloop = EventLoop::new();

    let window = WindowBuilder::new()
        .build(&evloop).unwrap();

    let mut input = WinitInputHelper::new();
    
    evloop.run(move |main_event, _, control_flow| {
        if input.update(&main_event) {
            // Update
        }

        match main_event {
            Event::WindowEvent {
                window_id,
                event: window_event
            } if window_id == window.id() => {
                match window_event {
                    WindowEvent::CloseRequested => { *control_flow = ControlFlow::Exit },
                    _ => {}
                }
            },
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                // Render
            },
            _ => {}
        }
    });
}
