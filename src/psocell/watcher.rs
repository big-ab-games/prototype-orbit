use super::PsoCell;
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

pub struct WatcherPsoCell<R: Resources, F: Factory<R>, I: pso::PipelineInit> {
    vertex_shader: PathBuf,
    fragment_shader: PathBuf,
    init: I,
    primitive: Primitive,
    raterizer: state::Rasterizer,
    _watcher: notify::RecommendedWatcher,
    shader_mods: mpsc::Receiver<notify::DebouncedEvent>,

    factory: F,
    pso: PipelineState<R, I::Meta>,
}

impl<R: Resources, F: Factory<R>, I: pso::PipelineInit + Clone> WatcherPsoCell<R, F, I> {
    fn recv_modified_pso(&mut self) -> Option<PipelineState<R, I::Meta>>
        where R: Resources,
              F: Factory<R>
    {
        if let Ok(notify::DebouncedEvent::NoticeWrite(path)) = self.shader_mods.try_recv() {
            match self.build_pso() {
                Ok(pso) => {
                    info!("{:?} changed", path);
                    return Some(pso);
                },
                Err(err) => error!("{:?}", err),
            };
        }
        None
    }

    fn build_pso(&mut self) -> Result<PipelineState<R, I::Meta>, Box<Error>>
        where R: Resources,
              F: Factory<R>
    {
        let fragment_shader = shader_bytes(&self.fragment_shader)?;

        let vertex_shader = shader_bytes(&self.vertex_shader)?;

        let set = self.factory.create_shader_set(&vertex_shader, &fragment_shader)?;
        Ok(self.factory
               .create_pipeline_state(&set,
                                      self.primitive,
                                      self.raterizer,
                                      self.init.clone())?)
    }
}

impl<R: Resources, F: Factory<R>, I: pso::PipelineInit + Clone> PsoCell<R, F, I> for WatcherPsoCell<R, F, I> {
    fn pso(&mut self) -> &mut PipelineState<R, I::Meta> {
        if let Some(updated) = self.recv_modified_pso() {
            self.pso = updated;
        }
        &mut self.pso
    }

    fn factory(&mut self) -> &mut F {
        &mut self.factory
    }
}

#[derive(Debug)]
pub struct WatcherPsoCellBuilder<I: pso::PipelineInit> {
    vertex_shader: Option<PathBuf>,
    fragment_shader: Option<PathBuf>,
    primitive: Primitive,
    raterizer: state::Rasterizer,
    init: I,
}

impl<I: pso::PipelineInit + Clone> WatcherPsoCellBuilder<I> {
    pub fn using(init_struct: I) -> WatcherPsoCellBuilder<I> {
        WatcherPsoCellBuilder {
            vertex_shader: None,
            fragment_shader: None,
            init: init_struct,
            primitive: Primitive::TriangleList,
            raterizer: state::Rasterizer::new_fill(),
        }
    }

    pub fn vertex_shader<P: Into<PathBuf>>(mut self, path: P) -> WatcherPsoCellBuilder<I> {
        self.vertex_shader = Some(path.into());
        self
    }

    pub fn fragment_shader<P: Into<PathBuf>>(mut self, path: P) -> WatcherPsoCellBuilder<I> {
        self.fragment_shader = Some(path.into());
        self
    }

    pub fn build<R, F>(self, mut factory: F)
            -> Result<WatcherPsoCell<R, F, I>, Box<Error>>
            where R: Resources, F: Factory<R> {
        let (tx, shader_mods) = mpsc::channel();
        let mut watcher = notify::watcher(tx, Duration::from_millis(100))?;
        let pso = {
            let vs = self.vertex_shader.as_ref().ok_or("missing vertex shader")?;
            let fs = self.fragment_shader.as_ref().ok_or("missing fragment shader")?;

            debug!("Watching {:?}", &[vs, fs]);
            watcher.watch(vs, notify::RecursiveMode::NonRecursive)?;
            watcher.watch(fs, notify::RecursiveMode::NonRecursive)?;

            let fragment_shader = shader_bytes(fs)?;
            let vertex_shader = shader_bytes(vs)?;
            let set = factory.create_shader_set(&vertex_shader, &fragment_shader)?;
            factory.create_pipeline_state(&set,
                                        self.primitive,
                                        self.raterizer,
                                        self.init.clone())?
        };

        Ok(WatcherPsoCell {
            vertex_shader: self.vertex_shader.ok_or("missing vertex shader")?,
            fragment_shader: self.fragment_shader.ok_or("missing fragment shader")?,
            init: self.init,
            primitive: self.primitive,
            raterizer: self.raterizer,
            _watcher: watcher,
            shader_mods,

            factory,
            pso,
        })
    }
}
