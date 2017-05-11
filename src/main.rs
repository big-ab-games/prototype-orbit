#[macro_use]
extern crate gfx;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate gfx_window_sdl;
extern crate sdl2;
extern crate time;
extern crate image;

use gfx::traits::{FactoryExt, Factory};
use gfx::{texture, Device};
use sdl2::event::Event as SdlEvent;
use sdl2::keyboard::Keycode;
use std::io::Cursor;

pub type ColorFormat = gfx::format::Rgba8;
pub type DepthFormat = gfx::format::DepthStencil;

gfx_defines!{
    vertex Vertex {
        pos: [f32; 2] = "position",
        color: [f32; 3] = "color",
        tex_coords: [f32; 2] = "tex_coords",
    }

    constant Uniforms {
        ms_ticks: f32 = "u_ticks",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        out: gfx::RenderTarget<ColorFormat> = "some_target",
        u_vals: gfx::ConstantBuffer<Uniforms> = "u_vals",
        t_happy: gfx::TextureSampler<[f32; 4]> = "t_happy",
        t_sad: gfx::TextureSampler<[f32; 4]> = "t_sad",
    }
}

const QUAD: [Vertex; 4] = [Vertex {
                               pos: [-0.5, 0.5],
                               color: [0.1, 0.3, 0.0],
                               tex_coords: [0.0, 0.0], // top-left
                           },
                           Vertex {
                               pos: [0.5, 0.5],
                               color: [0.2, 0.1, 1.0],
                               tex_coords: [1.0, 0.0], // top-right
                           },

                           Vertex {
                               pos: [0.5, -0.5],
                               color: [0.0, 1.0, 0.0],
                               tex_coords: [1.0, 1.0], // bottom-right
                           },
                           Vertex {
                               pos: [-0.5, -0.5],
                               color: [1.0, 0.1, 0.0],
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

pub fn main() {
    env_logger::init().unwrap();
    let sdl_context = sdl2::init().unwrap();
    let video = sdl_context.video().unwrap();

    video.gl_attr().set_context_profile(sdl2::video::GLProfile::Core);
    video.gl_attr().set_context_version(3, 3);
    video.gl_attr().set_stencil_size(8);
    video.gl_attr().set_accelerated_visual(true);
    let mut builder = video.window("Example", 1024, 768);
    builder.position(2560 / 2 + 100, 100); // for development purposes


    let (window, gl_context, mut device, mut factory, main_color, main_depth) =
        gfx_window_sdl::init::<ColorFormat, DepthFormat>(builder).unwrap();

    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();
    let happy_texture = load_texture(&mut factory, include_bytes!("img/screg_600_happy.png"));
    let sad_texture = load_texture(&mut factory, include_bytes!("img/screg_600_sad.png"));
    let sampler = factory.create_sampler(
        texture::SamplerInfo::new(texture::FilterMethod::Trilinear, texture::WrapMode::Mirror));

    let set = factory
        .create_shader_set(include_bytes!("shader/some.vert.glsl"),
                           include_bytes!("shader/some.frag.glsl"))
        .unwrap();
    let pso = factory
        .create_pipeline_state(&set,
                               gfx::Primitive::TriangleList,
                               gfx::state::Rasterizer {
                                   samples: Some(gfx::state::MultiSample {}),
                                   ..gfx::state::Rasterizer::new_fill()
                               },
                               pipe::new())
        .unwrap();

    let (vertex_buffer, slice) =
        factory.create_vertex_buffer_with_slice(&QUAD, &[0u16, 1, 2, 0, 2, 3] as &[u16]);

    let data = pipe::Data {
        vbuf: vertex_buffer,
        out: main_color,
        u_vals: factory.create_constant_buffer(1),
        t_happy: (happy_texture, sampler.clone()),
        t_sad: (sad_texture, sampler),
    };

    let start = time::precise_time_s() * 1000.0;

    'main: loop {
        let mut event_pump = sdl_context.event_pump().unwrap();

        for event in event_pump.poll_iter() {
            match event {
                SdlEvent::Quit { .. } |
                SdlEvent::KeyUp { keycode: Some(Keycode::Escape), .. } => {
                    info!("Quitting");
                    break 'main;
                }
                _ => {}
            }
        }
        encoder.update_constant_buffer(
            &data.u_vals,
            &Uniforms {
                ms_ticks: (time::precise_time_s() * 1000.0 - start) as f32,
            });
        encoder.clear(&data.out, CLEAR_COLOR);
        encoder.draw(&slice, &pso, &data);
        encoder.flush(&mut device);
        window.gl_swap_window();
        device.cleanup();
    }
}
