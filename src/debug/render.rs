use super::DebugInfo;
use gfx::*;
use gfx_text::{HorizontalAnchor, VerticalAnchor};
use gfx_text;

pub struct DebugInfoBrush<R: Resources, F: Factory<R>> {
    renderer: gfx_text::Renderer<R, F>
}

impl<R: Resources, F: Factory<R> + Clone> DebugInfoBrush<R, F> {
    pub fn new(factory: &F) -> DebugInfoBrush<R, F> {
        let renderer = gfx_text::new(factory.clone()).with_size(14).unwrap();
        DebugInfoBrush { renderer }
    }

    pub fn draw<C: CommandBuffer<R>, T: format::RenderFormat>(&mut self, encoder: &mut Encoder<R, C>, target: &handle::RenderTargetView<R, T>, info: &DebugInfo)
                      -> Result<(), gfx_text::Error>
    {
        let txt = format!("{} fps, {} cps", info.mean_fps, info.compute.mean_cps);
        self.renderer.add_anchored(&txt, [5, 5],
                       HorizontalAnchor::Left, VerticalAnchor::Top,
                       [0.3, 0.6, 0.8, 1.0]);
        self.renderer.draw(encoder, target)
    }
}
