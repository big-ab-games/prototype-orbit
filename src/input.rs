use glutin::*;
use state::*;

/// max zoom is mathematically the 'minimum' zoom value
const MAX_ZOOM: f32 = 0.5;

pub struct UserMouse {
    left_down: Option<(i32, i32)>,
    last_position: (i32, i32),
    // winit-next
    // left_down: Option<(f64, f64)>,
    // last_position: (f64, f64),
}

impl UserMouse {
    pub fn new() -> UserMouse {
        UserMouse {
            left_down: None,
            last_position: (0, 0)
        }
    }

    pub fn handle(&mut self, state: &mut UserState, _delta: f32, event: &WindowEvent) {
        match event {
            &WindowEvent::MouseWheel(MouseScrollDelta::LineDelta(_, y), ..) => {
            // winit-next
            // &WindowEvent::MouseWheel{ delta: MouseScrollDelta::LineDelta(_, y), ..} => {
                // general double/half zoom for fast view changes
                let factor = if y < 0. { state.zoom } else { state.zoom / 2. };


                let zoom_to = state.screen_to_world(self.last_position);
                state.zoom -= factor * y as f32;
                if state.zoom < MAX_ZOOM {
                    // enforce max zoom
                    state.zoom = MAX_ZOOM;
                }
                let zoomed_to = state.screen_to_world(self.last_position);
                state.origin += zoom_to - zoomed_to; // preserve pre-zoom cursor world position
                debug!("wheel:zoom -> {:.2} toward ({:.3},{:.3})", state.zoom, zoom_to.x, zoom_to.y);
            }
            &WindowEvent::MouseInput(ElementState::Pressed, MouseButton::Left) =>
            // winit-next
            // &WindowEvent::MouseInput{ state: ElementState::Pressed, button: MouseButton::Left, ..} =>
                self.left_down = Some(self.last_position),
            &WindowEvent::MouseInput(ElementState::Released, MouseButton::Left) => {
            // winit-next
            // &WindowEvent::MouseInput{ state: ElementState::Released, button: MouseButton::Left, ..} => {
                if self.left_down.is_some() {
                    debug!("left-drag {:?} -> {:?}", self.left_down.unwrap(), self.last_position);
                    self.left_down = None;
                }
            },
            &WindowEvent::MouseMoved(x, y) => {
            // winit-next
            // &WindowEvent::MouseMoved{ position: (x, y), ..} => {
                if self.left_down.is_some() {
                    let movement =
                        state.screen_to_world(self.last_position) - state.screen_to_world((x, y));
                    state.origin += movement;
                }
                self.last_position = (x, y);
            }
            _ => (),
        }
    }
}
