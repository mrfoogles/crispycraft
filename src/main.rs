#![allow(dead_code)]
use winit::{
    window::{WindowBuilder},
    event::*,
    event_loop::{EventLoop, ControlFlow}
};
use winit_input_helper::WinitInputHelper;

mod game;
mod terrain;

fn main() {
    let evloop = EventLoop::new();

    let window = WindowBuilder::new()
        .build(&evloop).unwrap();
    let wsize = window.inner_size();

    let mut input = WinitInputHelper::new();

    let mut world = terrain::TerrainState::new();
    let mut camera = game::camera::CameraData {
        eye: cgmath::point3(0.,18.,-2.),
        target: cgmath::point3(16.,0.,16.),
        up: cgmath::vec3(0.,1.,0.),

        aspect: wsize.width as f32 / wsize.height as f32,
        fovy: 70.,
        znear: 0.1,
        zfar: 40.
    };

    let wrl_mesh = world.make_mesh(1.);

    let mut state = pollster::block_on(game::State::new(&window, &wrl_mesh, &camera));

    evloop.run(move |main_event, _, control_flow| {
        if input.update(&main_event) {
            #[allow(unused_imports)]
            use cgmath::InnerSpace;
            // Update

            let sp = 0.1;
            let offset = camera.target - camera.eye;

            if input.key_held(VirtualKeyCode::A) {
                camera.eye.x += sp;
            }
            if input.key_held(VirtualKeyCode::D) {
                camera.eye.x -= sp;
            }
            camera.target = camera.eye + offset;
            state.update_camera(&camera);

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
