use svsc;
use glutin::*;
use std::thread;
use std::time::Duration;
use input::*;
use state::*;
use time;
use cgmath::*;
use orbitcurve::OrbitCurve;

const DESIRED_CPS: u32 = 1_080;
const DESIRED_DELTA: f64 = 1.0 / DESIRED_CPS as f64;

const GRAVITY: f64 = 0.01;

pub fn start(initial_state: State, events: EventsLoop) -> svsc::Getter<State> {
    let (latest_state_getter, render_state) = svsc::channel(initial_state.clone());

    thread::spawn(move|| {
        let mut tasks = Tasks::new();
        let mut user_mouse = UserMouse::new();
        let mut user_keys = UserKeys::new();

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
            compute_projections(&mut state, &tasks);

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

fn compute_projections(state: &mut State, tasks: &Tasks) {
    // remove current projections, re-initialise nil curves
    state.drawables.orbit_curves.clear();
    for body in state.drawables.orbit_bodies.iter() {
        let mut curve = OrbitCurve::new();
        curve.add_plot(body.center);
        state.drawables.orbit_curves.push(curve);
    }

    let mut f_tasks = tasks.clone();
    f_tasks.zoom = None;
    f_tasks.follow = None;
    let mut f_state = state.clone();

    let f_delta = 0.05;
    for _ in 0..1000 {
        compute_state(&mut f_state, &mut f_tasks, f_delta);
        for (idx, curve) in state.drawables.orbit_curves.iter_mut().enumerate() {
            let ref body = &f_state.drawables.orbit_bodies[idx];
            curve.add_plot(body.center);
        }
    }
    state.drawables.orbit_curves.retain(|ref c| c.is_drawable());
}
