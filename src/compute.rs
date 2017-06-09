use svsc;
use glutin::*;
use std::thread;
use std::time::Duration;
use input::*;
use state::*;
use time;
use cgmath::*;
use rayon::prelude::*;
use seer::*;

const DESIRED_CPS: u32 = 1_080;
const DESIRED_DELTA: f64 = 1.0 / DESIRED_CPS as f64;
const GRAVITY: f64 = 0.01;

#[cfg_attr(feature = "cargo-clippy", allow(float_cmp))]
pub fn start(initial_state: State, events: EventsLoop) -> svsc::Getter<State> {
    let (latest_state_getter, render_state) = svsc::channel(initial_state.clone());

    thread::spawn(move|| {
        let mut tasks = Tasks::new();
        let mut user_mouse = UserMouse::new();
        let mut user_keys = UserKeys::new();

        let mut seer = Seer::new(initial_state.clone(), tasks.clone());
        let mut seer_apprentice = None;

        let (mut delta_sum, mut delta_count) = (0.0, 0);
        let mut state = initial_state;
        let mut last_loop = time::precise_time_s();

        let mut mean_cps = DESIRED_CPS; // optimistic
        loop {
            let it_start = time::precise_time_s();
            let mut delta = it_start - last_loop;
            last_loop = it_start;
            if state.pause {
                delta = 0.0;
            }

            events.poll_events(|Event::WindowEvent{ event, .. }| {
                match event {
                    WindowEvent::KeyboardInput(_, _, Some(VirtualKeyCode::Escape), _) |
                    WindowEvent::Closed => state.user_quit = true,
                    _ => {}
                }
                user_mouse.handle(&mut state, delta as f32, &event, &mut tasks);
                user_keys.handle(&mut state, delta as f32, &event, &mut tasks);
            });

            compute_state(&mut state, &mut tasks, delta);

            handle_seer_projections(&mut state, &mut seer);

            if seer_apprentice.is_none() {
                let mut zoom = tasks.zoom.as_ref()
                    .map(|z| z.zoom_destination())
                    .unwrap_or(state.zoom);
                if zoom > state.zoom {
                    zoom = state.zoom;
                }
                // if we can tell the seer is losing his touch, ie his curves start erroneously
                // far from the orbit bodies, we spin up an apprentice in parallel seeded with
                // newer state. When the apprentice as 99% of the plots of his master we switch
                // to using the apprentice as the new seer
                if state.drawables.curve_body_mismatch(SEER_FAULT_TOLERANCE) {
                    debug!("Curve mismatch detected, getting an apprentice seer...");
                    seer_apprentice = Some(Seer::new(state.clone(), tasks.clone()));
                }
                // float comparison here works, as min_plot_distance_at_zoom returns from a set
                // of constants that are not modified
                else if seer.min_plot_distance != Seer::min_plot_distance_at_zoom(zoom) {
                    debug!("Zoom change needs seer plot accuracy, getting an apprentice seer...");
                    seer_apprentice = Some(Seer::new(state.clone(), tasks.clone()));
                }
            }
            else if let Some(mut apprentice) = seer_apprentice.take() {
                if !state.drawables.orbit_curves.is_empty() {
                    if apprentice.is_approx_as_good_as(&mut seer) {
                        debug!("Promoting apprentice seer");
                        seer = apprentice;
                    }
                    else { // still needs training
                        seer_apprentice = Some(apprentice);
                    }
                }
                else { // shouldn't happen
                    warn!("No curves...");
                }
            }

            seer.main_deltas.send(delta).expect("seer->delta");
            if let Some(ref apprentice) = seer_apprentice {
                apprentice.main_deltas.send(delta).expect("apprentice seer->delta");
            }

            delta_sum += delta;
            delta_count += 1;
            if delta_sum >= 1.0 { // ie update around every second
                mean_cps = (1.0 / (delta_sum / delta_count as f64)).round() as u32;
                delta_sum = 0.0;
                delta_count = 0;
            }
            state.debug_info.mean_cps = mean_cps;

            // update render state
            if render_state.update(state.clone()).is_err() {
                break; // rendering has finished / no getter
            }

            let sleep_delta = DESIRED_DELTA - (time::precise_time_s() - it_start);
            if sleep_delta > 0.0 {
                thread::sleep(Duration::new(0, (sleep_delta * 1_000_000_000.0) as u32));
            }
        }
    });

    latest_state_getter
}

fn compute_state_single(mut state: &mut State, tasks: &mut Tasks, delta: f64) {
    for idx in 0..state.drawables.orbit_bodies.len() {
        let mut new_velocity = state.drawables.orbit_bodies[idx].velocity;

        for idx2 in 0..state.drawables.orbit_bodies.len() {
            if idx != idx2 {
                let body = &state.drawables.orbit_bodies[idx];
                let other = &state.drawables.orbit_bodies[idx2];
                let dist_squared = body.center.distance2(other.center);
                let acceleration_scalar = GRAVITY * other.mass / dist_squared;
                let accelaration = (other.center - body.center).normalize_to(acceleration_scalar);

                new_velocity += delta * accelaration;
            }
        }

        state.drawables.orbit_bodies[idx].velocity = new_velocity;
    }

    tasks.update(&mut state);

    for body in &mut state.drawables.orbit_bodies {
        body.update(delta);
    }
}

fn compute_state_par(mut state: &mut State, tasks: &mut Tasks, delta: f64) {
    let bodies = state.drawables.orbit_bodies.clone();
    state.drawables.orbit_bodies.par_iter_mut().for_each(|body| {
        body.velocity += bodies.par_iter()
            .filter(|other| other.id != body.id)
            .map(|other| {
                let dist_squared = body.center.distance2(other.center);
                let acceleration_scalar = GRAVITY * other.mass / dist_squared;
                let accelaration = (other.center - body.center).normalize_to(acceleration_scalar);
                delta * accelaration
            })
            .sum();
    });

    tasks.update(&mut state);

    state.drawables.orbit_bodies.par_iter_mut().for_each(|body| {
        body.update(delta);
    });
}

pub fn compute_state(mut state: &mut State, tasks: &mut Tasks, delta: f64) {
    // benchmarks currently show > 16*4 bodies as the sweet spot for parallel impl
    if state.drawables.orbit_bodies.len() > 64 {
        compute_state_par(state, tasks, delta)
    }
    else {
        compute_state_single(state, tasks, delta)
    }
}

fn handle_seer_projections(state: &mut State, seer: &mut Seer) {
    state.drawables.orbit_curves = seer.projection.latest().clone();

    // fade between [10, 20]
    if state.zoom > 10.0 {
        let opacity = 1.0 - (state.zoom - 10.0) / 10.0;
        for curve in &mut state.drawables.orbit_curves {
            curve.opacity = opacity;
        }
    }
    else {
        for curve in &mut state.drawables.orbit_curves {
            curve.opacity = 1.0;
        }
    }
}

#[cfg(feature = "bench")]
#[cfg(test)]
mod compute_bench {
    use super::*;
    use uuid::Uuid;
    use orbitbody::*;
    use rayon;

    use test::Bencher;

    fn test_drawables() -> Drawables {
        Drawables {
            orbit_bodies: vec!(
                OrbitBody {
                    id: Uuid::new_v4(),
                    center: (0.0, 0.0).into(),
                    radius: 1.2,
                    mass: 1660.0,
                    velocity: (0.0, -1.0).into(),
                },
                OrbitBody {
                    id: Uuid::new_v4(),
                    center: (3.5, 0.0).into(),
                    radius: 0.9,
                    mass: 1000.0,
                    velocity: (0.0, 1.6).into(),
                },
                OrbitBody {
                    id: Uuid::new_v4(),
                    center: (9.0, 0.0).into(),
                    radius: 0.3,
                    mass: 1.0,
                    velocity: (0.0, 2.0).into(),
                },
                OrbitBody {
                    id: Uuid::new_v4(),
                    center: (-12.0, 0.0).into(),
                    radius: 0.4,
                    mass: 2.0,
                    velocity: (0.0, -1.5).into(),
                },
            ),
            orbit_curves: vec!(),
        }
    }

    fn setup_with_load(load: usize) -> State {
        let mut state = State::new(1980, 1440);
        state.drawables = Drawables {
            orbit_bodies: vec!(),
            orbit_curves: vec!(),
        };
        // 100x few bodies load
        for i in 0..load {
            for j in 0..load {
                for mut body in test_drawables().orbit_bodies {
                    body.center.x += i as f64 * 5.0;
                    body.center.y += j as f64 * 5.0;
                    state.drawables.orbit_bodies.push(body);
                }
            }
        }
        state
    }

    macro_rules! bench_single {
        ($name:ident, $load:expr) => {
            #[bench]
            fn $name(b: &mut Bencher) {
                let mut state = setup_with_load($load);
                let mut tasks = Tasks::new();
                b.iter(|| compute_state_single(&mut state, &mut tasks, 0.001));
            }
        }
    }

    macro_rules! bench_par {
        ($name:ident, $load:expr) => {
            #[bench]
            fn $name(b: &mut Bencher) {
                let mut state = setup_with_load($load);
                let mut tasks = Tasks::new();
                // err probably just means has already been called
                rayon::initialize(rayon::Configuration::new()).is_err();


                b.iter(|| compute_state_par(&mut state, &mut tasks, 0.001));
            }
        }
    }

    macro_rules! bench_default {
        ($name:ident, $load:expr) => {
            #[bench]
            fn $name(b: &mut Bencher) {
                let mut state = setup_with_load($load);
                let mut tasks = Tasks::new();
                // err probably just means has already been called
                rayon::initialize(rayon::Configuration::new()).is_err();

                b.iter(|| compute_state(&mut state, &mut tasks, 0.001));
            }
        }
    }

    // bench_single!(bench_compute_1x_bodies_single,   1);
    // bench_single!(bench_compute_9x_bodies_single,   3);
    // bench_single!(bench_compute_16x_bodies_single,  4);
    bench_single!(bench_compute_25x_bodies_single,  5);
    // bench_single!(bench_compute_36x_bodies_single,  6);
    // bench_single!(bench_compute_100x_bodies_single, 10);

    bench_par!(bench_compute_1x_bodies_par,   1);
    bench_par!(bench_compute_9x_bodies_par,   3);
    bench_par!(bench_compute_16x_bodies_par,  4);
    // bench_par!(bench_compute_25x_bodies_par,  5);
    // bench_par!(bench_compute_36x_bodies_par,  6);
    // bench_par!(bench_compute_100x_bodies_par, 10);

    bench_default!(bench_compute_1x_bodies_auto,   1);
    bench_default!(bench_compute_9x_bodies_auto,   3);
    bench_default!(bench_compute_16x_bodies_auto,  4);
    bench_default!(bench_compute_25x_bodies_auto,  5);
    bench_default!(bench_compute_36x_bodies_auto,  6);
    bench_default!(bench_compute_100x_bodies_auto, 10);
}
