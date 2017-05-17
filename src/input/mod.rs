use cgmath::*;
use glutin::*;

/// max zoom is mathematically the 'minimum' zoom value
const MAX_ZOOM: f32 = 0.1;

#[derive(Clone, Debug)]
pub struct UserState {
    pub origin: Vector2<f32>,
    pub zoom: f32,
    pub screen_width: u32,
    pub screen_height: u32,
    pub view: Matrix4<f32>,
    pub wants_out: bool,
}

fn bird_view_at_height(height: f32) -> Matrix4<f32> {
    let mut view = Matrix4::identity();
    view.z.z = height;
    view
}

impl UserState {
    pub fn new(screen_width: u32, screen_height: u32) -> UserState {
        UserState {
            origin: Vector2::new(0.0f32, 0.0),
            zoom: 1.0f32,
            screen_width,
            screen_height,
            view: bird_view_at_height(1.0),
            wants_out: false,
        }
    }

    pub fn ortho_projection(&self) -> Matrix4<f32> {
        ortho(self.origin.x - self.zoom * self.aspect_ratio(),
              self.origin.x + self.zoom * self.aspect_ratio(),
              self.origin.y - self.zoom,
              self.origin.y + self.zoom,
              0.0,
              100.0)
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.screen_width as f32 / self.screen_height as f32
    }
}

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
                state.zoom -= 0.1 * y as f32;
                if state.zoom < MAX_ZOOM {
                    // enforce max zoom
                    state.zoom = MAX_ZOOM;
                }
                debug!("wheel:zoom -> {}", state.zoom);
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
                    let (lastx, lasty) = self.last_position;

                    let (xrel, yrel) = (x - lastx, y - lasty);
                    state.origin +=
                        Vector2::new(-xrel as f32 * 2.0 * state.zoom * state.aspect_ratio() /
                                     state.screen_width as f32,
                                     (yrel as f32 * 2.0 * state.zoom) / state.screen_height as f32);
                }
                self.last_position = (x, y);
            }
            _ => (),
        }
    }
}
