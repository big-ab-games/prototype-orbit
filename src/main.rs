#[macro_use] extern crate log;
#[macro_use] extern crate gfx;
#[macro_use] extern crate gfx_macros;
extern crate pretty_env_logger;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate time;
extern crate image;
extern crate cgmath;
extern crate gfx_text;
extern crate notify;
extern crate easer;

mod input;
mod state;
mod svsc;
mod ease;

use std::sync::{Arc, Mutex};
use std::mem;
use gfx::traits::{FactoryExt, Factory};
use gfx::{texture, Device};
use glutin::*;
use std::io::Cursor;
use std::thread;
use gfx_text::{HorizontalAnchor, VerticalAnchor};
use notify::{Watcher, RecursiveMode, watcher};
use std::sync::mpsc::channel;
use std::time::Duration;
use std::path::Path;
use std::fs::File;
use std::error::Error;
use std::io::prelude::*;
use input::*;
use state::*;

pub type ColorFormat = gfx::format::Srgba8;
pub type DepthFormat = gfx::format::DepthStencil;

#[derive(VertexData, Debug, Clone)]
pub struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

gfx_defines! {
    constant Uniforms {
        ms_ticks: f32 = "u_ticks",
    }

    constant Transform {
        u_view: [[f32; 4]; 4] = "u_view",
        u_proj: [[f32; 4]; 4] = "u_proj",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        out: gfx::RenderTarget<ColorFormat> = "some_target",
        u_vals: gfx::ConstantBuffer<Uniforms> = "u_vals",
        u_transform: gfx::ConstantBuffer<Transform> = "u_transform",
        t_happy: gfx::TextureSampler<[f32; 4]> = "t_happy",
        t_sad: gfx::TextureSampler<[f32; 4]> = "t_sad",
    }
}

const QUAD: [Vertex; 4] = [Vertex {
                               position: [-0.5, 0.5],
                               tex_coords: [0.0, 0.0], // top-left
                           },
                           Vertex {
                               position: [0.5, 0.5],
                               tex_coords: [1.0, 0.0], // top-right
                           },

                           Vertex {
                               position: [0.5, -0.5],
                               tex_coords: [1.0, 1.0], // bottom-right
                           },
                           Vertex {
                               position: [-0.5, -0.5],
                               tex_coords: [0.0, 1.0], // bottom-left
                           }];

const CLEAR_COLOR: [f32; 4] = [0.05, 0.05, 0.05, 1.0];

pub fn load_texture<R, F>(factory: &mut F,
                          data: &[u8])
                          -> gfx::handle::ShaderResourceView<R, [f32; 4]>
                          where R: gfx::Resources,
                                F: gfx::Factory<R> {
    use gfx::texture as tex;
    let img = image::load(Cursor::new(data), image::PNG)
        .expect("!image::load")
        .to_rgba();
    let (width, height) = img.dimensions();
    let kind = tex::Kind::D2(width as tex::Size, height as tex::Size, tex::AaMode::Single);
    factory.create_texture_immutable_u8::<ColorFormat>(kind, &[&img])
        .expect("!create_texture_immutable_u8")
        .1
}

fn load_pipeline_state<R, F>(factory: &mut F) -> Result<gfx::PipelineState<R, pipe::Meta>, Box<Error>> where R: gfx::Resources,
      F: gfx::Factory<R> {
    let mut fragment_shader = Vec::new();
    File::open("src/shader/some.frag.glsl")?.read_to_end(&mut fragment_shader)?;
    let mut vertex_shader = Vec::new();
    File::open("src/shader/some.vert.glsl")?.read_to_end(&mut vertex_shader)?;
    let set = factory.create_shader_set(&vertex_shader, &fragment_shader)?;
    Ok(factory
        .create_pipeline_state(&set,
                               gfx::Primitive::TriangleList,
                               gfx::state::Rasterizer {
                                   samples: Some(gfx::state::MultiSample {}),
                                   ..gfx::state::Rasterizer::new_fill()
                               },
                               pipe::new())?)
}

pub fn main() {
    pretty_env_logger::init().unwrap();

    let events_loop = EventsLoop::new();
    let builder = WindowBuilder::new()
        .with_title("Example".to_string())
        .with_dimensions(1024, 768)
        .with_gl_profile(GlProfile::Core)
        .with_gl(GlRequest::Specific(Api::OpenGl, (3, 3)))
        .with_multisampling(8);

    let (window, mut device, mut factory, main_color, mut _main_depth) =
            gfx_window_glutin::init::<ColorFormat, DepthFormat>(builder, &events_loop);

    window.set_position(2560 / 2 + 100, 100); // for development purposes

    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();
    let happy_texture = load_texture(&mut factory, include_bytes!("img/screg_600_happy.png"));
    let sad_texture = load_texture(&mut factory, include_bytes!("img/screg_600_sad.png"));
    let sampler = factory.create_sampler(
        texture::SamplerInfo::new(texture::FilterMethod::Anisotropic(16), texture::WrapMode::Mirror));

    let mut pso = load_pipeline_state(&mut factory).expect("!load_pipeline_state");

    let mut fps_txt = gfx_text::new(factory.clone()).with_size(14).unwrap();

    let (vertex_buffer, slice) =
        factory.create_vertex_buffer_with_slice(&QUAD, &[0u16, 1, 2, 0, 2, 3] as &[u16]);

    let data = pipe::Data {
        vbuf: vertex_buffer,
        out: main_color.clone(),
        u_vals: factory.create_constant_buffer(1),
        u_transform: factory.create_constant_buffer(1),
        t_happy: (happy_texture, sampler.clone()),
        t_sad: (sad_texture, sampler),
    };

    let start = time::precise_time_s();
    let mut passed = time::precise_time_s() - start;
    let mut recent_frames = Vec::new();

    let (width_px, height_px) = window.get_inner_size_pixels().unwrap();
    let user_state = Arc::new(Mutex::new(UserState::new(width_px, height_px)));
    let (mut cps_get, render_cps) = svsc::channel(0u32);

    let c_user_state = user_state.clone();
    thread::spawn(move|| {
        let mut last_event = start;
        let mut tasks = Tasks::new();
        let mut user_mouse = UserMouse::new();

        const DESIRED_CPS: u32 = 1080;
        const DESIRED_DELTA: f64 = 1.0 / DESIRED_CPS as f64;
        let (mut delta_sum, mut delta_count) = (0.0, 0);

        loop {
            let it_start = time::precise_time_s();
            let delta = it_start - last_event;
            last_event = it_start;

            let mut user_lock = c_user_state.lock().unwrap();
            events_loop.poll_events(|Event::WindowEvent{window_id: _, event}| {
                match event {
                    WindowEvent::KeyboardInput(_, _, Some(VirtualKeyCode::Escape), _) |
                    WindowEvent::Closed => user_lock.wants_out = true,
                    _ => {}
                }
                user_mouse.handle(&mut *user_lock, delta as f32, &event, &mut tasks);
            });
            tasks.update(&mut *user_lock);
            // winit-next
            // events_loop.poll_events(|window_device_event| {
            //     if let Event::WindowEvent{ event, .. } = window_device_event {
            //         match event {
            //             WindowEvent::KeyboardInput {
            //                 input: KeyboardInput { virtual_keycode: Some(VirtualKeyCode::Escape), .. },
            //                 .. } |
            //             WindowEvent::Closed => user_lock.wants_out = true,
            //             _ => {}
            //         }
            //         user_mouse.handle(&mut *user_lock, delta as f32, &event);
            //     }
            // });
            mem::drop(user_lock);

            delta_sum += delta;
            delta_count += 1;
            if delta_count == DESIRED_CPS {
                let avg = delta_sum / DESIRED_CPS as f64;
                delta_sum = 0.0;
                delta_count = 0;
                if let Err(_) = render_cps.update((1.0 / avg).round() as u32) {
                    break; // rendering has finished / no getter
                }
            }

            let sleep_delta = DESIRED_DELTA - (time::precise_time_s() - it_start);
            if sleep_delta > 0.0 {
                thread::sleep(Duration::new(0, (sleep_delta * 1_000_000_000.0) as u32));
            }
        }
    });

    const DESIRED_FPS: u32 = 250;
    const DESIRED_DETLA: f64 = 1.0 / DESIRED_FPS as f64;

    let mut fps: i64 = 0;
    let (tx, shader_changes) = channel();
    let mut watcher = watcher(tx, Duration::from_millis(100)).unwrap();
    watcher.watch(Path::new("src/shader").canonicalize().unwrap(), RecursiveMode::Recursive).unwrap();

    loop {
        // reload shaders if changed
        if let Ok(notify::DebouncedEvent::NoticeWrite(path)) = shader_changes.try_recv() {
           info!("{:?} changed", path);
           match load_pipeline_state(&mut factory) {
               Ok(new_pso) => pso = new_pso,
               Err(err) => error!("{:?}", err)
           };
        }

        let last_passed = passed;
        passed = time::precise_time_s() - start;
        let delta = passed - last_passed;
        recent_frames.push(delta);

        let user_lock = user_state.lock().unwrap();
        if user_lock.wants_out {
            info!("Quitting");
            break;
        }

        if recent_frames.len() >= 250 {
            let sum: f64 = recent_frames.iter().sum();
            fps = (1.0 / (sum / recent_frames.len() as f64)).round() as i64;
            recent_frames.clear();
        }

        if fps > 0 {
            let mut txt = format!("{} fps", fps);

            let cps = cps_get.latest();
            if cps > &0 {
                txt += &format!(", {} cps", cps)
            }

            fps_txt.add_anchored(&txt, [5, 5],
                                 HorizontalAnchor::Left, VerticalAnchor::Top,
                                 [0.3, 0.6, 0.8, 1.0]);
        }

        encoder.update_constant_buffer(&data.u_vals,
                                       &Uniforms { ms_ticks: (passed * 1000.0) as f32 });

        encoder.update_constant_buffer(&data.u_transform, &Transform {
            u_view: user_lock.view.into(),
            u_proj: user_lock.projection().into(),
        });

        mem::drop(user_lock);

        encoder.clear(&data.out, CLEAR_COLOR);
        encoder.draw(&slice, &pso, &data);
        fps_txt.draw(&mut encoder, &main_color).unwrap();
        encoder.flush(&mut device);
        window.swap_buffers().unwrap();
        device.cleanup();

        let frame_time = time::precise_time_s() - start - passed;
        if DESIRED_DETLA - frame_time > 0.0 {
            thread::sleep(Duration::new(0, ((DESIRED_DETLA - frame_time) * 1_000_000_000.0) as u32));
        }
    }
}
