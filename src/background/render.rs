use super::{UserViewTransform, ColorFormat, DepthFormat};
use gfx::*;
use gfx::traits::FactoryExt;
use gfx;
use gfx_shader_watch::*;

#[derive(VertexData, Debug, Clone, Copy)]
pub struct BackgroundVertex {
    position: [f32; 2],
}

gfx_defines! {
    pipeline backgroundpipe {
        vbuf: VertexBuffer<BackgroundVertex> = (),
        out: RenderTarget<ColorFormat> = "out_color",
        out_depth: gfx::DepthTarget<DepthFormat> = gfx::preset::depth::LESS_EQUAL_WRITE,
        global_transform: ConstantBuffer<UserViewTransform> = "global_transform",
    }
}

const BACKGROUND_QUAD: [BackgroundVertex; 4] = [
    BackgroundVertex{ position: [-10000.0, 10000.0] },
    BackgroundVertex{ position: [10000.0, 10000.0] },
    BackgroundVertex{ position: [10000.0, -10000.0] },
    BackgroundVertex{ position: [-10000.0, -10000.0] }];

pub struct BackgroundBrush<R: Resources, F: Factory<R>> {
    pso_cell: debug_watcher_pso_cell_type!(R, F, pipe = backgroundpipe),
    slice: Slice<R>,
    data: backgroundpipe::Data<R>,
}

impl<R: Resources, F: Factory<R>> BackgroundBrush<R, F> {
    pub fn new(mut factory: F,
                  target: &handle::RenderTargetView<R, ColorFormat>,
                  depth_target: &handle::DepthStencilView<R, DepthFormat>)
                  -> BackgroundBrush<R, F>
    {
        let (vertex_buffer, slice) = factory
            .create_vertex_buffer_with_slice(&BACKGROUND_QUAD, &[0u16, 1, 2, 0, 2, 3] as &[u16]);
        let data = backgroundpipe::Data {
            vbuf: vertex_buffer,
            out: target.clone(),
            out_depth: depth_target.clone(),
            global_transform: factory.create_constant_buffer(1),
        };

        let pso_cell = debug_watcher_pso_cell!(
            pipe = backgroundpipe,
            vertex_shader = "shader/vert.glsl",
            fragment_shader = "shader/frag.glsl",
            factory = factory).expect("BackgroundBrush pso");

        BackgroundBrush { pso_cell, slice, data }
    }

    pub fn draw<C>(&mut self,
                      encoder: &mut Encoder<R, C>,
                      transform: &UserViewTransform) where C: CommandBuffer<R> {
        encoder.update_constant_buffer(&self.data.global_transform, transform);
        encoder.draw(&self.slice, self.pso_cell.pso(), &self.data);
    }
}
