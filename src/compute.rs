use svsc;
use glutin::*;
use std::thread;
use std::time::Duration;
use input::*;
use state::*;
use time;
use cgmath::*;
use orbitcurve::OrbitCurve;
use std::sync::mpsc;

const DESIRED_CPS: u32 = 1_080;
const DESIRED_DELTA: f64 = 1.0 / DESIRED_CPS as f64;

const GRAVITY: f64 = 0.01;

pub fn start(initial_state: State, events: EventsLoop) -> svsc::Getter<State> {
    let (latest_state_getter, render_state) = svsc::channel(initial_state.clone());

    thread::spawn(move|| {
        let mut tasks = Tasks::new();
        let mut user_mouse = UserMouse::new();
        let mut user_keys = UserKeys::new();

        let (computed_deltas, rx) = mpsc::channel();
        let mut seer = Seer::new(initial_state.clone(), rx);

        let (mut delta_sum, mut delta_count) = (0.0, 0);
        let mut state = initial_state;
        let mut last_loop = time::precise_time_s();

        let mut mean_cps = DESIRED_CPS; // optimistic
        loop {
            let it_start = time::precise_time_s();
            let delta = it_start - last_loop;
            last_loop = it_start;

            events.poll_events(|Event::WindowEvent{window_id: _, event}| {
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

            computed_deltas.send(delta).unwrap();

            delta_sum += delta;
            delta_count += 1;
            if delta_sum >= 1.0 { // ie update around every second
                mean_cps = (1.0 / (delta_sum / delta_count as f64)).round() as u32;
                delta_sum = 0.0;
                delta_count = 0;
            }
            state.debug_info.mean_cps = mean_cps;

            // update render state
            if let Err(_) = render_state.update(state.clone()) {
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

fn compute_state(mut state: &mut State, tasks: &mut Tasks, delta: f64) {
    for idx in 0..state.drawables.orbit_bodies.len() {
        let mut new_velocity = state.drawables.orbit_bodies[idx].velocity;

        for idx2 in 0..state.drawables.orbit_bodies.len() {
            if idx != idx2 {
                let ref body = state.drawables.orbit_bodies[idx];
                let ref other = state.drawables.orbit_bodies[idx2];
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

fn handle_seer_projections(state: &mut State, seer: &mut Seer) {
    state.drawables.orbit_curves = seer.projection.latest().clone();

    // fade between [10, 20]
    if state.zoom > 10.0 {
        let opacity = 1.0 - (state.zoom - 10.0) / 10.0;
        for curve in state.drawables.orbit_curves.iter_mut() {
            curve.opacity = opacity;
        }
    }
    else {
        for curve in state.drawables.orbit_curves.iter_mut() {
            curve.opacity = 1.0;
        }
    }
}

struct Seer {
    pub projection: svsc::Getter<Vec<OrbitCurve>>,
}

const SEER_COMPUTE_DELTA: f64 = 0.005;
const SEER_MAX_PLOTS: usize = 20_000;

impl Seer {
    fn new(initial_state: State, main_deltas_receiver: mpsc::Receiver<f64>) -> Seer {
        let (projection_get, projection) = svsc::channel(Vec::new());

        thread::spawn(move|| {
            let mut nil_tasks = Tasks::new();
            let mut plots = 0;
            let mut state = initial_state;
            let mut main_deltas_ahead = 0.0;

            let mut filtering = false;
            let (tx, filtered_updates) = mpsc::channel();

            state.drawables.orbit_curves.clear();
            for body in state.drawables.orbit_bodies.iter() {
                let mut curve = OrbitCurve::new();
                curve.plots.push(body.center);
                state.drawables.orbit_curves.push(curve);
            }

            loop {
                // consider main loop computed deltas and adjust
                while let Ok(delta) = main_deltas_receiver.try_recv() {
                    main_deltas_ahead += delta;
                }
                let outdated_plots = (main_deltas_ahead / SEER_COMPUTE_DELTA).floor();
                if outdated_plots > 0.0 {
                    main_deltas_ahead -= outdated_plots * SEER_COMPUTE_DELTA;
                    if plots > outdated_plots as usize {
                        plots -= outdated_plots as usize;
                    }
                    else {
                        plots = 0;
                    }
                    for curve in state.drawables.orbit_curves.iter_mut() {
                        curve.remove_oldest_plots(outdated_plots as usize);
                    }
                }

                if plots >= SEER_MAX_PLOTS {
                    thread::sleep(Duration::from_millis((SEER_COMPUTE_DELTA * 500.0).round() as u64));
                    continue;
                }

                compute_state(&mut state, &mut nil_tasks, SEER_COMPUTE_DELTA);
                for (idx, curve) in state.drawables.orbit_curves.iter_mut().enumerate() {
                    let ref body = &state.drawables.orbit_bodies[idx];
                    curve.plots.push(body.center);
                }
                plots += 1;

                if !filtering {
                    let curves = state.drawables.orbit_curves.clone();
                    let sender = tx.clone();
                    thread::spawn(move|| {
                        // filtering curves is quite intensive, so use another thread
                        let mut curves_for_render = Vec::new();
                        for curve in curves.iter() {
                            curves_for_render.push(curve.with_minimum_plot_distance(0.25));
                        }
                        sender.send(curves_for_render).unwrap();
                    });
                    filtering = true;
                }

                if let Ok(curves) = filtered_updates.try_recv() {
                    if let Err(_) = projection.update(curves) {
                        // dead getter
                        break;
                    }
                    filtering = false;
                }
            }
        });

        Seer { projection: projection_get }
    }
}
