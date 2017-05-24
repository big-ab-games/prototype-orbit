use svsc;
use glutin::*;
use std::thread;
use std::time::Duration;
use input::*;
use state::*;
use time;

pub fn start(initial_state: State, events: EventsLoop) -> svsc::Getter<State> {
    let (latest_state_getter, render_state) = svsc::channel(initial_state.clone());

    thread::spawn(move|| {
        let mut tasks = Tasks::new();
        let mut user_mouse = UserMouse::new();

        const DESIRED_CPS: u32 = 1080;
        const DESIRED_DELTA: f64 = 1.0 / DESIRED_CPS as f64;
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
            });
            tasks.update(&mut state);
            // winit-next
            // events_loop.poll_events(|window_device_event| {
            //     if let Event::WindowEvent{ event, .. } = window_device_event {
            //         match event {
            //             WindowEvent::KeyboardInput {
            //                 input: KeyboardInput { virtual_keycode: Some(VirtualKeyCode::Escape), .. },
            //                 .. } |
            //             WindowEvent::Closed => user_lock.wants_out = true,
            //             _ => {}
            //         }
            //         user_mouse.handle(&mut *user_lock, delta as f32, &event);
            //     }
            // });

            delta_sum += delta;
            delta_count += 1;
            if delta_count == DESIRED_CPS { // ie update around every second
                mean_cps = (1.0 / (delta_sum / DESIRED_CPS as f64)).round() as u32;
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
