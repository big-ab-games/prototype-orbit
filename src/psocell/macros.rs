

// #[cfg(debug_assertions)]
// type OrbitBodyPsoCell<R, F> = WatcherPsoCell<R, F, orbitbodypipe::Init<'static>>;
// #[cfg(not(debug_assertions))]
// type OrbitBodyPsoCell<R, F> = SimplePsoCell<R, F, orbitbodypipe::Init<'static>>;


#[cfg(debug_assertions)]
#[macro_export]
macro_rules! debug_watcher_pso_cell_type {
    ($r_type:ty, $f_type:ty, pipe = $pipe_name:ident) =>
        (WatcherPsoCell<$r_type, $f_type, $pipe_name::Init<'static>>)
}

#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! debug_watcher_pso_cell_type {
    ($r_type:ty, $f_type:ty, pipe = $pipe_name:ident) =>
        (SimplePsoCell<$r_type, $f_type, $pipe_name::Init<'static>>)
}


// #[cfg(debug_assertions)]
// let pso = WatcherPsoCellBuilder::using(orbitbodypipe::new())
//     .vertex_shader("src/orbitbody/shader/vert.glsl")
//     .fragment_shader("src/orbitbody/shader/frag.glsl")
//     .build(factory)
//     .expect("OrbitBodyBrush pso");
//
// #[cfg(not(debug_assertions))]
// let pso = SimplePsoCellBuilder::using(orbitbodypipe::new())
//     .vertex_shader(include_bytes!("shader/vert.glsl"))
//     .fragment_shader(include_bytes!("shader/frag.glsl"))
//     .build(factory)
//     .expect("OrbitBodyBrush pso");

#[cfg(debug_assertions)]
#[macro_export]
macro_rules! debug_watcher_pso_cell {
    (pipe = $pipe_name:ident,
    vertex_shader = $vs:expr,
    fragment_shader = $fs:expr,
    factory = $factory:expr) => {{
        use std::path::Path;
        match Path::new(file!()).canonicalize() {
            Ok(path) => match path.parent().ok_or("Could not find current dir") {
                Ok(dir) => {
                    let vs = dir.join($vs);
                    let fs = dir.join($fs);
                    WatcherPsoCellBuilder::using($pipe_name::new())
                        .vertex_shader(vs)
                        .fragment_shader(fs)
                        .build($factory)
                },
                Err(err) => Err(err.into())
            },
            Err(err) => Err(err.into())
        }
    }}
}

#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! debug_watcher_pso_cell {
    (pipe = $pipe_name:ident,
    vertex_shader = $vs:expr,
    fragment_shader = $fs:expr,
    factory = $factory:expr) => {{
        SimplePsoCellBuilder::using($pipe_name::new())
            .vertex_shader(include_bytes!($vs))
            .fragment_shader(include_bytes!($fs))
            .build($factory)
    }}
}
