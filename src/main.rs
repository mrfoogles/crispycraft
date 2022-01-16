//#![allow(dead_code)]
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

    // has methods like .key_held(VirtualKeyCode::W)
    let mut input = WinitInputHelper::new();

    // The chunks
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

    const MAX_FACES: u32 = 16 * 16 * 16 * 6 / 2;
    const MAX_VERTS: u32 = MAX_FACES * 4; // four points
    const MAX_INDXS: u32 = MAX_FACES * 6; // two triangles(3) from the points
    // State requires an initial mesh - uploading a mesh requires MAX_VERTS & MAX_INDXS becuase you can't resize buffers
    let mut state = pollster::block_on(game::State::new(&window, &wrl_mesh, MAX_VERTS / 8, MAX_INDXS / 8, &camera));

    evloop.run(move |main_event, _, control_flow| {
        // Input also checks for some special events, which is why we update only when it says so
        if input.update(&main_event) {
            #[allow(unused_imports)]
            use cgmath::InnerSpace; // Useful methods on vector3s
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

            // The code renders on the RedrawRequested event, but normally that's only sent once, then on resizes.
            //  this makes it send the RedrawRequested event every frame, as well.
            window.request_redraw();
        }

        match main_event {
            Event::WindowEvent {
                window_id,
                event: window_event
            } if window_id == window.id() => {
                match window_event {
                    // Close when you press the red button on the window
                    WindowEvent::CloseRequested => { *control_flow = ControlFlow::Exit },
                    // Resize correctly
                    WindowEvent::Resized(new_size) => {
                        state.resize(new_size);
                    },
                    _ => {}
                }
            },
            // Let the OS request us to re-render whenever it needs to
            Event::RedrawRequested(window_id) if window_id == window.id() => {
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
