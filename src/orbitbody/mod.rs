pub mod render;

use super::*;
use cgmath::*;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct OrbitBody {
    pub id: Uuid,
    pub center: Vector2<f64>,
    pub radius: f64,
    pub mass: f64,
    pub velocity: Vector2<f64>,
}

impl OrbitBody {
    pub fn update(&mut self, delta: f64) {
        self.center += self.velocity * delta;
    }
}
