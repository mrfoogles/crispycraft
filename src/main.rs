//#![allow(dead_code)]
use winit::{
    window::{WindowBuilder},
    event::*,
    event_loop::{EventLoop, ControlFlow}
};
use winit_input_helper::WinitInputHelper;

mod game;
use game::BindGroupSource;
mod terrain;
use terrain::ChunkPos;

use block_mesh::ndshape::ConstShape;

fn main() {
    let evloop = EventLoop::new();

    let window = WindowBuilder::new()
        .build(&evloop).unwrap();
    let wsize = window.inner_size();

    let mut ctx = pollster::block_on(game::WgpuCtx::default(&window));

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
        zfar: 400.
    };

    let generate = |[lx,ly,lz]: [i32; 3], [x,y,z]: [i32; 3]| {
        use terrain::{Block,SIZE};

        if lx == 0 || lx == 17 || ly == 0 || ly == 17 || lz == 0 || lz == 17 {
            return Block { solid: false }
        };

        if (y as f32) < (x as f32 / 16. + z as f32 / 8.).sin() * 8. + 8. {
            Block { solid: true }
        } else {
            Block { solid: false }
        }
    };

    fn make_mesh<F: Fn(ChunkPos, ChunkPos) -> terrain::Block>(
        world: &mut terrain::TerrainState, 
        ctx: &mut game::WgpuCtx,
        renderer: &mut game::ChunkRender,
        pos: terrain::ChunkPos,
        generator: F
    ) {
        world.set_chunk(pos, generator);
        renderer.cache_chunk_mesh(
            &ctx, pos, 
            &terrain::ChunkShape {}, 
            world.chunks.get(&pos).unwrap(), 
            terrain::SIZE, 1.
        );
    }

    let mut chunk_r = game::ChunkRender::new(&ctx, &camera, terrain::ChunkShape::SIZE as usize);
    let chunks = (-2..2).flat_map(|x| {
        (-2..2).map(|z| {
            [x as i32, 0, z as i32]
        }).collect::<Vec<terrain::ChunkPos>>()
    }).collect::<Vec<terrain::ChunkPos>>();
    for chunk in chunks.iter() { make_mesh(&mut world, &mut ctx, &mut chunk_r, *chunk, generate); }

    let (camera_group, camera_buffer) = camera.bind_group(&ctx.device, &ctx.queue, &camera.bind_group_layout(&ctx.device));
    let mut depth_texture = game::texture::Texture::create_depth_texture(&ctx.device, &ctx.config, "depth tex");

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
            camera.update_bind_group(&camera_buffer, &ctx.queue);

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
                        ctx.resize(new_size);
                        depth_texture = game::texture::Texture::create_depth_texture(&ctx.device, &ctx.config, "depth tex");
                    },
                    _ => {}
                }
            },
            // Let the OS request us to re-render whenever it needs to
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                match chunk_r.render(&ctx, &depth_texture, &camera_group, &chunks) {
                    Ok(_) => {}
                    // Reconfigure the surface if lost
                    Err(wgpu::SurfaceError::Lost) => {
                        ctx.resize(ctx.size);
                        depth_texture = game::texture::Texture::create_depth_texture(&ctx.device, &ctx.config, "depth tex");
                    },
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
