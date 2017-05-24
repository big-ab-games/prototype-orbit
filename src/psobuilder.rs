use gfx::traits::FactoryExt;
use gfx::*;
use std::path::PathBuf;
use std::fs::File;
use std::error::Error;
use std::io::prelude::*;
use std::sync::mpsc;
use std::time::Duration;
use notify;
use notify::Watcher;

fn shader_bytes(path: &PathBuf) -> Result<Vec<u8>, Box<Error>> {
    let mut shader = Vec::new();
    File::open(path)?.read_to_end(&mut shader)?;
    Ok(shader)
}

pub struct PsoBuilder<I: pso::PipelineInit> {
    vertex_shader: Option<PathBuf>,
    fragment_shader: Option<PathBuf>,
    init: Option<I>,
    primitive: Primitive,
    raterizer: state::Rasterizer,
}

impl<I: pso::PipelineInit + Clone> PsoBuilder<I> {
    pub fn new() -> PsoBuilder<I> {
        PsoBuilder {
            vertex_shader: None,
            fragment_shader: None,
            init: None,
            primitive: Primitive::TriangleList,
            raterizer: state::Rasterizer {
                // samples: Some(state::MultiSample {}),
                ..state::Rasterizer::new_fill()
            },
        }
    }

    pub fn vertex_shader<P: Into<PathBuf>>(mut self, path: P) -> PsoBuilder<I> {
        self.vertex_shader = Some(path.into());
        self
    }

    pub fn fragment_shader<P: Into<PathBuf>>(mut self, path: P) -> PsoBuilder<I> {
        self.fragment_shader = Some(path.into());
        self
    }

    pub fn init_struct(mut self, init: I) -> PsoBuilder<I> {
        self.init = Some(init);
        self
    }

    // TODO non-default raterizer/primitive

    pub fn build_with<R: Resources, F: Factory<R>>
        (&self,
         factory: &mut F)
         -> Result<PipelineState<R, I::Meta>, Box<Error>> {
        let fragment_shader = shader_bytes(self.fragment_shader
                                               .as_ref()
                                               .ok_or("missing fragment shader")?)?;

        let vertex_shader = shader_bytes(self.vertex_shader
                                             .as_ref()
                                             .ok_or("missing fragment shader")?)?;

        let set = factory
            .create_shader_set(&vertex_shader, &fragment_shader)?;
        Ok(factory
               .create_pipeline_state(&set,
                                      self.primitive,
                                      self.raterizer,
                                      self.init.clone().ok_or("missing init struct")?)?)
    }

    pub fn watch<P: Into<PathBuf>>(self, watch_path: P) -> PsoWatcher<I> {
        let (tx, shader_mods) = mpsc::channel();
        let mut watcher = notify::watcher(tx, Duration::from_millis(100)).unwrap();
        watcher
            .watch(watch_path.into().canonicalize().unwrap(), notify::RecursiveMode::Recursive)
            .unwrap();
        PsoWatcher {
            builder: self,
            _watcher: watcher,
            shader_mods,
        }
    }
}

pub struct PsoWatcher<I: pso::PipelineInit> {
    builder: PsoBuilder<I>,
    _watcher: notify::RecommendedWatcher,
    shader_mods: mpsc::Receiver<notify::DebouncedEvent>,
}

impl<I: pso::PipelineInit + Clone> PsoWatcher<I> {
    pub fn recv_modified<R, F>(&self, factory: &mut F) -> Option<PipelineState<R, I::Meta>> where R: Resources, F: Factory<R> {
        if let Ok(notify::DebouncedEvent::NoticeWrite(path)) = self.shader_mods.try_recv() {
            info!("{:?} changed", path);
            match self.build_with(factory) {
                Ok(pso) => return Some(pso),
                Err(err) => error!("{:?}", err),
            };
        }
        None
    }

    pub fn build_with<R: Resources, F: Factory<R>>(&self, factory: &mut F)
            -> Result<PipelineState<R, I::Meta>, Box<Error>> {
        self.builder.build_with(factory)
    }
}
