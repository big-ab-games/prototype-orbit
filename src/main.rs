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
extern crate num_traits;

mod input;
mod state;
mod svsc;
mod background;
mod orbitbody;
mod ease;
mod psobuilder;
mod compute;
mod debug;

use gfx::{Device};
use glutin::*;
use std::io::Cursor;
use std::thread;
use std::time::Duration;
use state::*;
use orbitbody::OrbitBody;

pub type ColorFormat = gfx::format::Srgba8;
pub type DepthFormat = gfx::format::Depth;


gfx_defines! {
    constant Time {
        ms_ticks: f32 = "ticks",
    }

    constant Transform {
        view: [[f32; 4]; 4] = "view",
        proj: [[f32; 4]; 4] = "proj",
    }
}

const CLEAR_COLOR: [f32; 4] = [0.05, 0.05, 0.05, 0.0];

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

pub fn main() {
    pretty_env_logger::init().unwrap();

    let events_loop = EventsLoop::new();
    let builder = WindowBuilder::new()
        .with_title("Example".to_string())
        .with_dimensions(1024, 768)
        .with_gl_profile(GlProfile::Core)
        .with_gl(GlRequest::Specific(Api::OpenGl, (3, 3)))
        .with_multisampling(8);

    let (window, mut device, mut factory, main_color, main_depth) =
            gfx_window_glutin::init::<ColorFormat, DepthFormat>(builder, &events_loop);

    window.set_position(2560 / 2 + 100, 100); // for development purposes

    let (width_px, height_px) = window.get_inner_size_pixels().unwrap();

    // Compute logic in seperate thread(s)
    let mut state_get = compute::start(State::new(width_px, height_px), events_loop);
    let start = time::precise_time_s();

    // Render logic in main thread
    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();
    let mut orbit_body_brush = orbitbody::render::OrbitBodyBrush::new(&mut factory, &main_color, &main_depth);
    let mut background_brush = background::render::BackgroundBrush::new(&mut factory, &main_color, &main_depth);
    let mut debug_info_brush = debug::render::DebugInfoBrush::new(&factory);

    const DESIRED_FPS: u32 = 250;
    const DESIRED_DETLA: f64 = 1.0 / DESIRED_FPS as f64;
    let (mut delta_sum, mut delta_count) = (0.0, 0);
    let mut passed = time::precise_time_s() - start;

    let mut mean_fps = DESIRED_FPS; // optimistic
    loop {
        let last_passed = passed;
        passed = time::precise_time_s() - start;
        let delta = passed - last_passed;

        let state = state_get.latest();
        if state.user_quit {
            info!("Quitting");
            break;
        }

        let projection = state.projection();
        let view = state.view.clone();

        let time = Time { ms_ticks: (passed * 1000.0) as f32 };

        encoder.clear(&main_color, CLEAR_COLOR);
        encoder.clear_depth(&main_depth, 1.0);

        let transform = Transform {
            view: view.into(),
            proj: projection.into(),
        };

        orbit_body_brush.draw(&mut factory, &mut encoder, &time, &transform,
            &state.drawables.orbit_bodies);
        background_brush.draw(&mut factory, &mut encoder, &transform);

        delta_sum += delta;
        delta_count += 1;
        if delta_count == DESIRED_FPS { // ie update around every second
            mean_fps = (1.0 / (delta_sum / DESIRED_FPS as f64)).round() as u32;
            delta_sum = 0.0;
            delta_count = 0;
        }

        debug_info_brush.draw(&mut encoder, &main_color, &state.debug_info.add_render_info(mean_fps))
            .unwrap();
        encoder.flush(&mut device);
        window.swap_buffers().unwrap();
        device.cleanup();


        let frame_time = time::precise_time_s() - start - passed;
        if DESIRED_DETLA - frame_time > 0.0 {
            thread::sleep(Duration::new(0, ((DESIRED_DETLA - frame_time) * 1_000_000_000.0) as u32));
        }
    }
}
