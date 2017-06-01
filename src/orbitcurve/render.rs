use super::super::{UserViewTransform, ColorFormat, DepthFormat};
use super::OrbitCurve;
use cgmath::*;
use gfx::*;
use gfx::traits::FactoryExt;
use gfx;
use gfx_shader_watch::*;

const LINE_WIDTH: f32 = 0.1;

#[derive(VertexData, Debug, Clone, Copy)]
pub struct OrbitCurveVertex {
    position: [f32; 2],
    local_idx: u32,
}

impl OrbitCurveVertex {
    fn new<V: Into<[f32; 2]>>(pos: V, index: usize) -> OrbitCurveVertex {
        OrbitCurveVertex {
            position: pos.into(),
            local_idx: index as u32,
        }
    }
}

#[derive(ConstantBuffer, Debug, Clone, Copy)]
pub struct OrbitCurveBezier {
    p1: [f32; 2],
    p2: [f32; 2],
    opacity: f32,
    thickness: f32,
    std140_offset: [u32; 2],
}

gfx_defines! {
    pipeline orbitbodypipe {
        vbuf: VertexBuffer<OrbitCurveVertex> = (),
        out: BlendTarget<ColorFormat> = ("out_color", state::ColorMask::all(), preset::blend::ALPHA),
        out_depth: gfx::DepthTarget<DepthFormat> = preset::depth::LESS_EQUAL_WRITE,
        global_transform: ConstantBuffer<UserViewTransform> = "global_transform",
        beziers: ConstantBuffer<OrbitCurveBezier> = "beziers",
    }
}

pub struct OrbitCurveBrush<R: Resources, F: Factory<R>> {
    pso_cell: debug_watcher_pso_cell_type!(R, F, pipe = orbitbodypipe),
    slice: Slice<R>,
    data: orbitbodypipe::Data<R>,
}

struct WorldView {
    min: Vector2<f32>,
    max: Vector2<f32>,
}

impl WorldView {
    fn contains(&self, p: Vector2<f32>) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }

    fn with_extra(&self, val: f32) -> WorldView {
        WorldView {
            min: Vector2::new(self.min.x - val, self.min.y - val),
            max: Vector2::new(self.max.x + val, self.max.y + val),
        }
    }
}

impl<R: Resources, F: Factory<R>> OrbitCurveBrush<R, F> {
    pub fn new(mut factory: F,
               target: &handle::RenderTargetView<R, ColorFormat>,
               depth_target: &handle::DepthStencilView<R, DepthFormat>)
               -> OrbitCurveBrush<R, F>
    {
        let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(&[], ());
        let data = orbitbodypipe::Data {
            vbuf: vertex_buffer,
            out: target.clone(),
            out_depth: depth_target.clone(),
            global_transform: factory.create_constant_buffer(1),
            beziers: factory.create_constant_buffer(0),
        };

        let pso_cell = debug_watcher_pso_cell!(
            pipe = orbitbodypipe,
            vertex_shader = "shader/vert.glsl",
            fragment_shader = "shader/frag.glsl",
            factory = factory,
            primitive = Primitive::TriangleStrip).expect("OrbitCurve pso");

        OrbitCurveBrush { pso_cell, slice, data }
    }

    pub fn draw<C>(&mut self,
                   encoder: &mut Encoder<R, C>,
                   transform: &UserViewTransform,
                   curve: &OrbitCurve,
                   (visible_min, visible_max): (Vector2<f32>, Vector2<f32>))
                   where C: CommandBuffer<R> {
        if curve.opacity < 0.00001 || !curve.is_drawable() {
            return;
        }

        // let curve = curve.with_minimum_plot_distance(0.3);
        // if !curve.is_drawable() {
        //     return;
        // }

        let view = WorldView { min: visible_min, max: visible_max }.with_extra(LINE_WIDTH);

        encoder.update_constant_buffer(&self.data.global_transform, transform);
        self.data.beziers = self.pso_cell.factory().create_constant_buffer(curve.plots.len() - 1);

        let cavg = curve.mean_plot().cast();
        let mut all_verts = Vec::with_capacity((curve.plots.len() - 1) * 2 + 2);

        let c1 = curve.plots[0].cast();
        let c1_to_cavg = (cavg - c1).normalize();

        let v1 = OrbitCurveVertex::new(c1 - c1_to_cavg * LINE_WIDTH / 2.0, 0);
        let v2 = OrbitCurveVertex::new(c1 + c1_to_cavg * LINE_WIDTH / 2.0, 0);
        all_verts.push(v1);
        all_verts.push(v2);

        for plot_idx in 0..(curve.plots.len()-1) {
            let c1 = curve.plots[plot_idx].cast();
            let c2 = curve.plots[plot_idx + 1].cast();

            if !view.contains(c1) && !view.contains(c2) {
                if (plot_idx == 0 || !view.contains(curve.plots[plot_idx-1].cast())) &&
                    (plot_idx == curve.plots.len()-2 || !view.contains(curve.plots[plot_idx+2].cast())) {
                    // Current points, and neighbours are outsite the current view, so skip
                    continue;
                }
            }

            let c2_to_cavg = (cavg - c2).normalize();

            let v3 = OrbitCurveVertex::new(c2 - c2_to_cavg * LINE_WIDTH / 2.0, plot_idx + 1);
            let v4 = OrbitCurveVertex::new(c2 + c2_to_cavg * LINE_WIDTH / 2.0, plot_idx + 1);

            all_verts.push(v3);
            all_verts.push(v4);

            let bezier = OrbitCurveBezier {
                p1: c1.into(),
                p2: c2.into(),
                opacity: curve.opacity * (1.0 - (plot_idx+1) as f32 / (curve.plots.len()-1) as f32),
                thickness: LINE_WIDTH,
                std140_offset: [0; 2],
            };
            encoder.update_buffer(&self.data.beziers, &[bezier], plot_idx).unwrap();
        }

        let (vertex_buffer, slice) =
            self.pso_cell.factory().create_vertex_buffer_with_slice(all_verts.as_slice(), ());
        self.data.vbuf = vertex_buffer;
        self.slice = slice;

        encoder.draw(&self.slice, self.pso_cell.pso(), &self.data);
    }
}
