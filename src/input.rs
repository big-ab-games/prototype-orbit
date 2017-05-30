use glutin::*;
use state::*;
use ease::*;
use cgmath::*;
use time;
use easer::functions::*;

const MIN_ZOOM: f32 = 0.5;
const MAX_ZOOM: f32 = 70.0;

#[derive(Clone, Debug)]
pub struct Zoomer {
    easer: Easer<f32>,
}

impl Zoomer {
    pub fn zoom_to_screen(zoom: f32, sceen_location: (i32, i32), current: &State) -> Zoomer {
        let mut after_state = current.clone();
        after_state.zoom = zoom;
        let zoom_to = current.screen_to_world(sceen_location);
        let zoomed_to = after_state.screen_to_world(sceen_location);
        let new_origin = current.origin + zoom_to - zoomed_to;

        Zoomer {
            easer: Easer::using(Expo::ease_out)
                    .start(time::precise_time_s() as f32)
                    .duration(1.0)
                    .add_transition(current.zoom, zoom)
                    .add_transition(current.origin.x, new_origin.x)
                    .add_transition(current.origin.y, new_origin.y)
        }
    }

    pub fn zoom_to_world(zoom: f32, world_location: (f32, f32), current: &State) -> Zoomer {
        Zoomer {
            easer: Easer::using(Expo::ease_out)
                    .start(time::precise_time_s() as f32)
                    .duration(1.0)
                    .add_transition(current.zoom, zoom)
                    .add_transition(current.origin.x, world_location.0)
                    .add_transition(current.origin.y, world_location.1)
        }
    }

    /// :return still zooming (ie unfinsihed)
    pub fn update(&self, state: &mut State) -> bool {
        let now = time::precise_time_s() as f32;
        let vals = self.easer.values_at(now);
        state.zoom = vals[0];
        state.origin = Vector2::new(vals[1], vals[2]);
        !self.easer.has_finished(now)
    }

    pub fn zoom_destination(&self) -> f32 {
        self.easer.transitions[0].1
    }
}

pub struct Tasks {
    pub zoom: Option<Zoomer>,
}

impl Tasks {
    pub fn new() -> Tasks {
        Tasks { zoom: None }
    }

    pub fn update(&mut self, mut state: &mut State) {
        if let Some(zoomer) = self.zoom.take() {
            if zoomer.update(&mut state) {
                self.zoom = Some(zoomer);
            }
        }
    }
}

pub struct UserMouse {
    left_down: Option<(i32, i32)>,
    last_position: (i32, i32),
}

impl UserMouse {
    pub fn new() -> UserMouse {
        UserMouse {
            left_down: None,
            last_position: (0, 0)
        }
    }

    pub fn handle(&mut self, state: &mut State, _delta: f32, event: &WindowEvent, tasks: &mut Tasks) {
        match event {
            &WindowEvent::MouseWheel(MouseScrollDelta::LineDelta(_, y), ..) => {
                // general double/half zoom for fast view changes
                let mut current_zoom = state.zoom;
                if let Some(ref zoomer) = tasks.zoom {
                    current_zoom = zoomer.zoom_destination();
                }

                let factor = if y < 0. { current_zoom } else { current_zoom / 2. };
                let mut new_zoom = current_zoom - factor * y as f32;
                if new_zoom < MIN_ZOOM {
                    new_zoom = MIN_ZOOM;
                }
                else if new_zoom > MAX_ZOOM {
                    new_zoom = MAX_ZOOM;
                }
                tasks.zoom = Some(Zoomer::zoom_to_screen(new_zoom, self.last_position, &state));
                debug!("wheel:zooming {:.2} -> {:.2} toward ({:.3},{:.3})",
                    state.zoom, new_zoom, self.last_position.0, self.last_position.1);
            }
            &WindowEvent::MouseInput(ElementState::Pressed, MouseButton::Left) => {
                self.left_down = Some(self.last_position);
                tasks.zoom = None; // cancel any current zooming
            },
            &WindowEvent::MouseInput(ElementState::Released, MouseButton::Left) => {
                if self.left_down.is_some() {
                    debug!("left-drag {:?} -> {:?}", self.left_down.unwrap(), self.last_position);
                    self.left_down = None;
                }
            },
            &WindowEvent::MouseMoved(x, y) => {
                if self.left_down.is_some() {
                    let movement =
                        state.screen_to_world(self.last_position) - state.screen_to_world((x, y));
                    state.origin += movement;
                }
                self.last_position = (x, y);
            },
            _ => (),
        }
    }
}

pub struct UserKeys {}

impl UserKeys {
    pub fn new() -> UserKeys {
        UserKeys {}
    }

    pub fn handle(&mut self, state: &mut State, _delta: f32, event: &WindowEvent, tasks: &mut Tasks) {
        match event {
            &WindowEvent::KeyboardInput(_, _, Some(VirtualKeyCode::Home), _) => {
                tasks.zoom = Some(Zoomer::zoom_to_world(state.zoom, (0f32, 0f32), &state));
            },
            _ => ()
        }
    }
}
