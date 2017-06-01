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

        let mut seer = Seer::new(initial_state.clone());
        let mut seer_apprentice = None;

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

            if seer_apprentice.is_none() {
                // if we can tell the seer is losing his touch, ie his curves start erroneously
                // far from the orbit bodies, we spin up an apprentice in parallel seeded with
                // newer state. When the apprentice as 99% of the plots of his master we switch
                // to using the apprentice as the new seer
                if state.drawables.curve_body_mismatch(SEER_FAULT_TOLERANCE) {
                    debug!("Curve mismatch detected, getting an apprentice seer...");
                    seer_apprentice = Some(Seer::new(state.clone()));
                }
                else if seer.min_plot_distance != Seer::min_plot_distance_at_zoom(state.zoom) {
                    debug!("Zoom change needs seer plot accuracy, getting an apprentice seer...");
                    seer_apprentice = Some(Seer::new(state.clone()));
                }
            }
            else if let Some(mut apprentice) = seer_apprentice.take() {
                if state.drawables.orbit_curves.len() > 0 {
                    if apprentice.is_approx_as_good_as(&mut seer) {
                        debug!("Apprentice seer is ready, he's the new seer");
                        seer = apprentice;
                    }
                    else { // still needs training
                        seer_apprentice = Some(apprentice);
                    }
                }
                else {
                    // shouldn't happen
                    debug!("No curves...");
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
    pub main_deltas: mpsc::Sender<f64>,
    pub min_plot_distance: f64,
}

const SEER_COMPUTE_DELTA: f64 = 0.001;
const SEER_MAX_PLOTS: usize = 50_000;
const SEER_FAULT_TOLERANCE: f64 = 0.5;

use uuid::Uuid;

impl Seer {
    fn min_plot_distance_at_zoom(zoom: f32) -> f64 {
        return if zoom >= 8.1 { 0.3 }
            else if zoom >= 2.1 { 0.2 }
            else if zoom >= 1.1 { 0.1 }
            else { 0.05 }
    }

    fn is_approx_as_good_as(&mut self, other: &mut Seer) -> bool {
        let plots = self.projection.latest().get(0)
            .map(|c| c.plots.len())
            .unwrap_or(0) as f64 * self.min_plot_distance;
        let other_plots = other.projection.latest().get(0)
            .map(|c| c.plots.len())
            .unwrap_or(0)  as f64 * other.min_plot_distance;

        plots >= other_plots * 0.99
    }

    fn new(initial_state: State) -> Seer {
        let min_plot_distance = Seer::min_plot_distance_at_zoom(initial_state.zoom);
        let (tx, main_deltas_receiver) = mpsc::channel();
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

            let me = Uuid::new_v4();
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
                    if projection.dead_getter() {
                        break; // dead getter, we've been forgotten
                    }
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
                            curves_for_render.push(curve.with_minimum_plot_distance(min_plot_distance));
                        }

                        if let Err(_) = sender.send(curves_for_render) {
                            // err => seer is forgotten, not interesting
                        }
                    });
                    filtering = true;
                }

                if let Ok(curves) = filtered_updates.try_recv() {
                    if let Err(_) = projection.update(curves) {
                        break; // dead getter, we've been forgotten
                    }
                    filtering = false;
                }
            }
            debug!("Seer {} forgetten", me);
        });

        Seer {
            projection: projection_get,
            main_deltas: tx,
            min_plot_distance,
        }
    }
}
