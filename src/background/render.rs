use super::{UserViewTransform, ColorFormat, DepthFormat};
use gfx::*;
use gfx::traits::FactoryExt;
use gfx;
use psobuilder::{PsoBuilder, PsoWatcher};

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

pub struct BackgroundBrush<R: Resources> {
    pso: PipelineState<R, backgroundpipe::Meta>,
    slice: Slice<R>,
    data: backgroundpipe::Data<R>,

    pso_builder: PsoWatcher<backgroundpipe::Init<'static>>,
}

impl<R: Resources> BackgroundBrush<R> {
    pub fn new<F>(factory: &mut F,
                  target: &handle::RenderTargetView<R, ColorFormat>,
                  depth_target: &handle::DepthStencilView<R, DepthFormat>)
                  -> BackgroundBrush<R>
        where F: Factory<R>
    {
        let pso_builder = PsoBuilder::new()
            .vertex_shader("src/background/shader/vert.glsl")
            .fragment_shader("src/background/shader/frag.glsl")
            .init_struct(backgroundpipe::new())
            .watch("src/background/shader");

        let pso = pso_builder.build_with(factory).expect("OrbitBodyDrawer initial pso");

        let (vertex_buffer, slice) = factory
            .create_vertex_buffer_with_slice(&BACKGROUND_QUAD, &[0u16, 1, 2, 0, 2, 3] as &[u16]);
        let data = backgroundpipe::Data {
            vbuf: vertex_buffer,
            out: target.clone(),
            out_depth: depth_target.clone(),
            global_transform: factory.create_constant_buffer(1),
        };

        BackgroundBrush { pso, slice, data, pso_builder }
    }

    pub fn draw<F, C>(&mut self,
                      factory: &mut F,
                      encoder: &mut Encoder<R, C>,
                      transform: &UserViewTransform) where F: Factory<R>, C: CommandBuffer<R> {
        // reload shaders if changed
        if let Some(pso) = self.pso_builder.recv_modified(factory) {
            self.pso = pso;
        }
        encoder.update_constant_buffer(&self.data.global_transform, transform);
        encoder.draw(&self.slice, &self.pso, &self.data);
    }
}
