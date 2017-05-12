#[macro_use] extern crate log;
#[macro_use] extern crate gfx;
#[macro_use] extern crate gfx_macros;
extern crate env_logger;
extern crate gfx_window_sdl;
extern crate sdl2;
extern crate time;
extern crate image;
extern crate cgmath;
extern crate gfx_text;

// use std::f64::consts::PI;
use gfx::traits::{FactoryExt, Factory};
use gfx::{texture, Device};
use sdl2::event::Event as SdlEvent;
use sdl2::keyboard::Keycode;
use std::io::Cursor;
use cgmath::*;
use gfx_text::{HorizontalAnchor, VerticalAnchor};

pub type ColorFormat = gfx::format::Srgba8;
pub type DepthFormat = gfx::format::DepthStencil;

#[derive(VertexData, Debug, Clone)]
pub struct Vertex {
    position: [f32; 2],
    color: [f32; 3],
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
                               color: [0.1, 0.3, 0.0],
                               tex_coords: [0.0, 0.0], // top-left
                           },
                           Vertex {
                               position: [0.5, 0.5],
                               color: [0.2, 0.1, 1.0],
                               tex_coords: [1.0, 0.0], // top-right
                           },

                           Vertex {
                               position: [0.5, -0.5],
                               color: [0.0, 1.0, 0.0],
                               tex_coords: [1.0, 1.0], // bottom-right
                           },
                           Vertex {
                               position: [-0.5, -0.5],
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

fn bird_view_at_height(height: f32) -> Matrix4<f32> {
    let mut view = Matrix4::identity();
    view.z.z = 1.5f32;
    view
}

fn ortho_projection(aspect: f32, zoom: f32, origin: Vector2<f32>) -> Matrix4<f32> {
    ortho(origin.x - zoom * aspect, origin.x + zoom * aspect,
          origin.y - zoom , origin.y + zoom, 0.0, 100.0)
}

pub fn main() {
    env_logger::init().unwrap();
    let sdl_context = sdl2::init().unwrap();
    let video = sdl_context.video().unwrap();

    video.gl_attr().set_context_profile(sdl2::video::GLProfile::Core);
    video.gl_attr().set_context_version(3, 3);
    video.gl_attr().set_stencil_size(8);
    video.gl_attr().set_accelerated_visual(true);
    video.gl_attr().set_multisample_samples(8); // AA
    let mut builder = video.window("Example", 1024, 768);

    builder.position(2560 / 2 + 100, 100); // for development purposes


    let (window, gl_context, mut device, mut factory, main_color, main_depth) =
        gfx_window_sdl::init::<ColorFormat, DepthFormat>(builder).unwrap();

    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();
    let happy_texture = load_texture(&mut factory, include_bytes!("img/screg_600_happy.png"));
    let sad_texture = load_texture(&mut factory, include_bytes!("img/screg_600_sad.png"));
    let sampler = factory.create_sampler(
        texture::SamplerInfo::new(texture::FilterMethod::Anisotropic(16), texture::WrapMode::Mirror));

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
    let mut view: Matrix4<f32> = bird_view_at_height(1.0);

    // let perspective_projection = perspective(Rad((PI * 0.3) as f32), 1024. / 768., 0.01, 10.0);
    let aspect = 1024./768.;
    let mut zoom = 1.0f32;
    let mut origin = Vector2::new(0.0f32, 0.0);
    let mut fps: i64 = 0;

    'main: loop {
        let last_passed = passed;
        passed = time::precise_time_s() - start;
        let delta = passed - last_passed;
        let deltaf = delta as f32;
        recent_frames.push(delta);

        let mut event_pump = sdl_context.event_pump().unwrap();
        for event in event_pump.poll_iter() {
            match event {
                SdlEvent::Quit { .. } |
                SdlEvent::KeyUp { keycode: Some(Keycode::Escape), .. } => {
                    info!("Quitting");
                    break 'main;
                }
                SdlEvent::KeyDown { keycode: Some(Keycode::Up), .. } => {
                    origin.y += 3. * deltaf;
                    info!("origin -> {:?}", origin)
                }
                SdlEvent::KeyDown { keycode: Some(Keycode::Down), .. } => {
                    origin.y -= 3. * deltaf;
                    info!("origin -> {:?}", origin)
                }
                SdlEvent::KeyDown { keycode: Some(Keycode::Left), .. } => {
                    origin.x -= 3. * deltaf;
                    info!("origin -> {:?}", origin)
                }
                SdlEvent::KeyDown { keycode: Some(Keycode::Right), .. } => {
                    origin.x += 3. * deltaf;
                    info!("origin -> {:?}", origin)
                }
                SdlEvent::MouseWheel { y, .. } => {
                    zoom -= 5. * deltaf * y as f32;
                    if zoom < 0.1 { // enforce max zoom
                        zoom = 0.1f32;
                    }
                    info!("zoom -> {}", zoom);
                }
                _ => {}
            }
        }

        if recent_frames.len() >= 20 {
            let sum: f64 = recent_frames.iter().sum();
            fps = (1.0 / (sum / recent_frames.len() as f64)).round() as i64;
            // debug!("{} fps", (1.0 / (sum / recent_frames.len() as f64)).round() as i64);
            recent_frames.clear();
        }

        if fps > 0 {
            fps_txt.add_anchored(&format!("{} fps", fps), [5, 5],
                                 HorizontalAnchor::Left, VerticalAnchor::Top,
                                 [0.3, 0.6, 0.8, 1.0]);
        }

        encoder.update_constant_buffer(&data.u_vals,
                                       &Uniforms { ms_ticks: (passed * 1000.0) as f32 });

        encoder.update_constant_buffer(&data.u_transform, &Transform {
            u_view: view.into(),
            u_proj: ortho_projection(aspect, zoom, origin).into(),
        });

        encoder.clear(&data.out, CLEAR_COLOR);
        encoder.draw(&slice, &pso, &data);
        fps_txt.draw(&mut encoder, &main_color).unwrap();
        encoder.flush(&mut device);
        window.gl_swap_window();

        device.cleanup();
    }
}
