use glutin::*;
use state::*;
use ease::*;
use cgmath::*;
use time;
use easer::functions::*;
use uuid::Uuid;
use std::time::{Instant, Duration};

const MIN_ZOOM: f32 = 0.5;
const MAX_ZOOM: f32 = 70.0;

const DBL_CLICK_MS: u64 = 500;

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

    pub fn just_zoom(zoom: f32, current: &State) -> Zoomer {
        Zoomer {
            easer: Easer::using(Expo::ease_out)
                    .start(time::precise_time_s() as f32)
                    .duration(1.0)
                    .add_transition(current.zoom, zoom)
                    .add_transition(current.origin.x, current.origin.x)
                    .add_transition(current.origin.y, current.origin.y)
        }
    }

    pub fn zoom_destination(&self) -> f32 {
        self.easer.transitions[0].1
    }

    pub fn zoom_at(&self, time: f32) -> f32 {
        let vals = self.easer.values_at(time);
        vals[0]
    }

    pub fn origin_at(&self, time: f32) -> Vector2<f32> {
        let vals = self.easer.values_at(time);
        Vector2::new(vals[1], vals[2])
    }

    pub fn finished_at(&self, time: f32) -> bool {
        self.easer.has_finished(time)
    }

    pub fn update_origin_destination<V: Into<(f32, f32)>>(&mut self, new: V) {
        let (newx, newy) = new.into();
        self.easer.transitions[1].1 = newx;
        self.easer.transitions[2].1 = newy;
    }
}

#[derive(Clone, Debug)]
pub struct Tasks {
    pub zoom: Option<Zoomer>,
    pub follow: Option<Uuid>,
}

impl Tasks {
    pub fn new() -> Tasks {
        Tasks { zoom: None, follow: None }
    }

    pub fn update(&mut self, mut state: &mut State) {
        let mut following = None;
        if let Some(id) = self.follow.take() {
            following = state.drawables.orbit_bodies.iter().find(|b| b.id == id);
        }

        if let Some(mut zoomer) = self.zoom.take() {
            if let Some(body) = following {
                zoomer.update_origin_destination(body.center.cast());
                self.follow = Some(body.id);
            }
            let now = time::precise_time_s() as f32;
            state.zoom = zoomer.zoom_at(now);
            state.origin = zoomer.origin_at(now);
            if !zoomer.finished_at(now) {
                self.zoom = Some(zoomer);
            }
        }
        else if let Some(body) = following {
            state.origin = (body.center.x as f32, body.center.y as f32).into();
            self.follow = Some(body.id);
        }
    }
}

#[derive(Clone, Debug)]
pub struct UserMouse {
    left_down: Option<(i32, i32)>,
    last_position: (i32, i32),
    last_left_click: Instant,
}

impl UserMouse {
    pub fn new() -> UserMouse {
        UserMouse {
            left_down: None,
            last_position: (0, 0),
            // init in past sometime, to avoid optional complexity
            last_left_click: Instant::now() - Duration::from_secs(999)
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
                // cancel any current tasks
                tasks.zoom = None;
                tasks.follow = None;
                if self.last_left_click.elapsed() < Duration::from_millis(DBL_CLICK_MS) {
                    self.handle_double_click(state, tasks);
                }
                self.last_left_click = Instant::now();
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

    fn handle_double_click(&mut self, state: &mut State, tasks: &mut Tasks) {
        let click_pos = state.screen_to_world(self.last_position);
        debug!("dbl click at {:?} => world {:?}", self.last_position, click_pos);
        let body = state.drawables.orbit_bodies.iter().find(|body| {
            click_pos.distance(body.center.cast()) < body.radius as f32
        });
        if let Some(body) = body {
            info!("Following body {}", body.id);
            tasks.zoom = Some(Zoomer::just_zoom(state.zoom, state));
            tasks.follow = Some(body.id);
        }
    }
}

#[derive(Clone, Debug)]
pub struct UserKeys {}

impl UserKeys {
    pub fn new() -> UserKeys {
        UserKeys {}
    }

    pub fn handle(&mut self, state: &mut State, _delta: f32, event: &WindowEvent, tasks: &mut Tasks) {
        match event {
            &WindowEvent::KeyboardInput(ElementState::Pressed, _, Some(keypress), _) => {
                let body = match keypress {
                    VirtualKeyCode::Home => state.drawables.orbit_bodies.iter()
                        .max_by_key(|x| x.mass.round() as i64),
                    VirtualKeyCode::Key1 => state.drawables.orbit_bodies.get(0),
                    VirtualKeyCode::Key2 => state.drawables.orbit_bodies.get(1),
                    VirtualKeyCode::Key3 => state.drawables.orbit_bodies.get(2),
                    VirtualKeyCode::Key4 => state.drawables.orbit_bodies.get(3),
                    VirtualKeyCode::Key5 => state.drawables.orbit_bodies.get(4),
                    VirtualKeyCode::Key6 => state.drawables.orbit_bodies.get(5),
                    VirtualKeyCode::Key7 => state.drawables.orbit_bodies.get(6),
                    VirtualKeyCode::Key8 => state.drawables.orbit_bodies.get(7),
                    VirtualKeyCode::Key9 => state.drawables.orbit_bodies.get(8),
                    VirtualKeyCode::Key0 => state.drawables.orbit_bodies.get(9),
                    _ => None
                };
                if let Some(body) = body {
                    tasks.follow = None;
                    tasks.zoom = Some(Zoomer::zoom_to_world(state.zoom,
                                                            body.center.cast().into(),
                                                            &state));
                }
            },
            _ => ()
        }
    }
}
