use super::{UserViewTransform, ColorFormat, DepthFormat, OrbitBody};
use cgmath::Matrix4;
use gfx::*;
use gfx::traits::FactoryExt;
use gfx;
use gfx_shader_watch::*;

#[derive(VertexData, Debug, Clone, Copy)]
pub struct OrbitBodyVertex {
    position: [f32; 2],
    local_idx: u32,
}

#[derive(ConstantBuffer, Debug, Clone, Copy)]
pub struct OrbitBodyTransform {
    transform: [[f32; 4]; 4],
}

gfx_defines! {
    pipeline orbitbodypipe {
        vbuf: VertexBuffer<OrbitBodyVertex> = (),
        out: BlendTarget<ColorFormat> = ("out_color", state::ColorMask::all(), preset::blend::ALPHA),
        out_depth: gfx::DepthTarget<DepthFormat> = preset::depth::LESS_EQUAL_WRITE,
        global_transform: ConstantBuffer<UserViewTransform> = "global_transform",
        local_transform: ConstantBuffer<OrbitBodyTransform> = "local_transform",
    }
}

impl OrbitBodyTransform {
    fn new(transform: [[f32; 4]; 4]) -> OrbitBodyTransform {
        OrbitBodyTransform {
            transform,
        }
    }
}

// equilateral triangle with incircle radius 1, and incircle center (0, 0)
// ref: https://rechneronline.de/pi/equilateral-triangle.php
//    C
//   /\
//  /  \
// /____\
// A     B
//
// right-angle tri A, center, midAB
//    (0,0)
//   /| radius
//  /_|
// A  midAB
//
// A<->midAB = 6 / 2√3
// midAB<->center = 1
// A<->center = √((6 / 2√3)^2 + 1^2) = √(36 / 12 + 1) = 2
//
// A: (-6 / 2√3, -1)
// B: (6 / 2√3, -1)
// C: (0, 2)

const ROOT3: f64 = 1.7320508075688774;
const BX: f64 = (6.0 / (2.0 * ROOT3));

const ORBIT_BODY_VERTICES: [OrbitBodyVertex; 3] = [
    OrbitBodyVertex{ position: [0.0, 2.0], local_idx: 0 },
    OrbitBodyVertex{ position: [-BX as f32, -1.0], local_idx: 0 },
    OrbitBodyVertex{ position: [BX as f32, -1.0], local_idx: 0 }];

pub struct OrbitBodyBrush<R: Resources, F: Factory<R>> {
    pso_cell: debug_watcher_pso_cell_type!(R, F, pipe = orbitbodypipe),
    slice: Slice<R>,
    data: orbitbodypipe::Data<R>,
}

impl<R: Resources, F: Factory<R>> OrbitBodyBrush<R, F> {
    pub fn new(mut factory: F,
               target: &handle::RenderTargetView<R, ColorFormat>,
               depth_target: &handle::DepthStencilView<R, DepthFormat>)
               -> OrbitBodyBrush<R, F>
    {
        let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(&[], ());
        let data = orbitbodypipe::Data {
            vbuf: vertex_buffer,
            out: target.clone(),
            out_depth: depth_target.clone(),
            global_transform: factory.create_constant_buffer(1),
            local_transform: factory.create_constant_buffer(0),
        };

        let pso_cell = debug_watcher_pso_cell!(
            pipe = orbitbodypipe,
            vertex_shader = "shader/vert.glsl",
            fragment_shader = "shader/frag.glsl",
            factory = factory).expect("OrbitBody pso");

        OrbitBodyBrush { pso_cell, slice, data }
    }

    pub fn draw<C>(&mut self,
                      encoder: &mut Encoder<R, C>,
                      transform: &UserViewTransform,
                      bodies: &[OrbitBody]) where C: CommandBuffer<R> {
        encoder.update_constant_buffer(&self.data.global_transform, transform);

        if self.data.vbuf.len() != bodies.len() * 3 {
            self.data.local_transform = self.pso_cell.factory().create_constant_buffer(bodies.len());

            let mut all_verts = Vec::with_capacity(bodies.len() * 3);
            for body_idx in 0..bodies.len() {
                for vert in &mut ORBIT_BODY_VERTICES {
                    vert.local_idx = body_idx as u32;
                    all_verts.push(*vert);
                }
            }
            let (vertex_buffer, slice) = self.pso_cell.factory().create_vertex_buffer_with_slice(all_verts.as_slice(), ());
            self.data.vbuf = vertex_buffer;
            self.slice = slice;
        }

        for (idx, body) in bodies.iter().enumerate() {
            let locals = OrbitBodyTransform::new(local_transform(body));
            encoder.update_buffer(&self.data.local_transform, &[locals], idx).expect("OrbitBody draw");
        }

        encoder.draw(&self.slice, self.pso_cell.pso(), &self.data);
    }
}

fn local_transform(body: &OrbitBody) -> [[f32; 4]; 4] {
    let scale = Matrix4::from_nonuniform_scale(body.radius as f32, body.radius as f32, 1.0);
    let translate = Matrix4::from_translation([body.center.x as f32, body.center.y as f32, 0.0].into());
    (translate * scale).into()
}
