use cgmath::*;
use std::fmt::Debug;
use std::any::Any;
use ease::*;
use time;

#[derive(Clone, Debug)]
pub struct UserState {
    pub origin: Vector2<f32>,
    pub zoom: f32,
    pub screen_width: u32,
    pub screen_height: u32,
    pub view: Matrix4<f32>,
    pub wants_out: bool,
}

fn birds_eye_at_z(height: f32) -> Matrix4<f32> {
    let mut view = Matrix4::identity();
    view.z.z = height;
    view
}

impl<'a> UserState {
    pub fn new(screen_width: u32, screen_height: u32) -> UserState {
        UserState {
            origin: Vector2::new(0.0f32, 0.0),
            zoom: 1.0f32,
            screen_width,
            screen_height,
            view: birds_eye_at_z(1.0),
            wants_out: false,
        }
    }

    pub fn projection(&self) -> Matrix4<f32> {
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

    /// translates screen pixels into world co-ordinates in the orthographic projection
    pub fn screen_to_world<V: Into<Vector2<i32>>>(&self, pixels: V) -> Vector2<f32> {
        let pixels = pixels.into();
        let x_world = self.zoom * self.aspect_ratio() * (pixels.x as f32 * 2.0 / self.screen_width as f32 - 1f32);
        let y_world = self.zoom * (-pixels.y as f32 * 2.0 / self.screen_height as f32 + 1f32);
        Vector2::new(x_world, y_world)
    }
}

#[cfg(test)]
mod state_test {
    use super::*;

    // see https://github.com/gfx-rs/gfx/tree/master/src/backend/gl
    // (0,0)
    //     ┌─┐
    //     └─┘
    //        (width-px, height-px)
    //      |
    //      v
    // (-1az,1z)
    //     ┌─┐
    //     └─┘
    //        (1az,-1z)
    // :a aspect ratio
    // :z zoom
    fn test_screen_to_world(s: UserState) {
        let a = s.aspect_ratio();
        let z = s.zoom;
        assert_eq!(s.screen_to_world(Vector2::new(0, 0)),
            Vector2::new(-a * z, 1f32 * z), "top-left");
        assert_eq!(s.screen_to_world(Vector2::new(s.screen_width as i32, 0)),
            Vector2::new(a * z, 1f32 * z), "top-right");
        assert_eq!(s.screen_to_world(Vector2::new(0, s.screen_height as i32)),
            Vector2::new(-a * z, -1f32 * z), "bottom-left");
        assert_eq!(s.screen_to_world(Vector2::new(s.screen_width as i32, s.screen_height as i32)),
            Vector2::new(a * z, -1f32 * z), "bottom-right");
        assert_eq!(s.screen_to_world(Vector2::new(s.screen_width as i32 / 2, s.screen_height as i32 / 2 )),
            Vector2::new(0f32, 0f32), "center");
    }
    #[test]
    fn screen_to_world_zoom_1_aspect_1() {
        //      | simplifies to
        //      v
        // (-1,1)
        //     ┌─┐
        //     └─┘
        //        (1,-1)
        test_screen_to_world(UserState::new(100, 100));
    }

    #[test]
    fn screen_to_world_zoom_1() {
        //      | simplifies to
        //      v
        // (-1a,1)
        //     ┌─┐
        //     └─┘
        //        (1a,-1)
        test_screen_to_world(UserState::new(160, 90));
    }

    #[test]
    fn screen_to_world() {
        //      | simplifies to
        //      v
        // (-1a,1)
        //     ┌─┐
        //     └─┘
        //        (1a,-1)
        let mut state = UserState::new(160, 90);
        state.zoom = 0.33f32;
        test_screen_to_world(state);
    }
}
