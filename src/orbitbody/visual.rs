use super::{Transform, Time, ColorFormat, OrbitBody, load_pipeline_state};
use cgmath::Matrix4;
use gfx::*;
use std::cell::RefCell;
use gfx::traits::FactoryExt;
use gfx;

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
    pipeline pipe {
        vbuf: VertexBuffer<OrbitBodyVertex> = (),
        out: RenderTarget<ColorFormat> = "out_color",
        time: ConstantBuffer<Time> = "time",
        global_transform: ConstantBuffer<Transform> = "global_transform",
        local_transform: ConstantBuffer<OrbitBodyTransform> = "local_transform",
    }
}

const ORBIT_BODY_VERTICES: [OrbitBodyVertex; 3] = [
    OrbitBodyVertex{ position: [0.0,0.5], local_idx: 0 },
    OrbitBodyVertex{ position: [-0.5,-0.5], local_idx: 0 },
    OrbitBodyVertex{ position: [0.5,-0.5], local_idx: 0 }];

pub struct OrbitBodyDrawer<R: Resources> {
    pso: PipelineState<R, pipe::Meta>,
    slice: Slice<R>,
    data: pipe::Data<R>,
}

impl<R: Resources> OrbitBodyDrawer<R> {
    pub fn new<F>(factory: &mut F,
                  target: handle::RenderTargetView<R, ColorFormat>)
                  -> OrbitBodyDrawer<R>
        where F: Factory<R>
    {
        let pso = load_pipeline_state(factory, pipe::new()).expect("!load_pipeline_state");

        // let arr = [Vertex{ position: [0.0,0.5]}, Vertex{ position: [-0.5,-0.5]}, Vertex{ position: [0.5,-0.5]}];
        let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(&[], ());
        let data = pipe::Data {
            vbuf: vertex_buffer,
            out: target,
            time: factory.create_constant_buffer(1),
            global_transform: factory.create_constant_buffer(1),
            local_transform: factory.create_constant_buffer(0),
        };

        OrbitBodyDrawer { pso, slice, data }
    }

    pub fn draw<F, C>(&mut self,
                      factory: &mut F,
                      encoder: &mut Encoder<R, C>,
                      time: &Time,
                      transform: &Transform,
                      bodies: &[OrbitBody]) where F: Factory<R>, C: CommandBuffer<R> {
        encoder.update_constant_buffer(&self.data.time, time);
        encoder.update_constant_buffer(&self.data.global_transform, transform);

        if self.data.vbuf.len() != bodies.len() * 3 {
            self.data.local_transform = factory.create_constant_buffer(bodies.len());

            let mut all_verts = Vec::with_capacity(bodies.len() * 3);
            for body_idx in 0..bodies.len() {
                let verts = ORBIT_BODY_VERTICES;
                for i in 0..verts.len() {
                    let mut vert = verts[i];
                    vert.local_idx = body_idx as u32;
                    all_verts.push(vert);
                }
            }
            info!("creating vertices: {:?}", all_verts);
            let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(all_verts.as_slice(), ());
            self.data.vbuf = vertex_buffer;
            self.slice = slice;
        }

        for idx in 0..bodies.len() {
            encoder.update_buffer(&self.data.local_transform,
                &[OrbitBodyTransform{ transform: local_transform(&bodies[idx]) }], idx).unwrap();
        }

        encoder.draw(&self.slice, &self.pso, &self.data);
    }
}

fn local_transform(body: &OrbitBody) -> [[f32; 4]; 4] {
    Matrix4::from_translation([body.center.x as f32, body.center.y as f32, 0.0].into()).into()
}
