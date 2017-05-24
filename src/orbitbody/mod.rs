pub mod render;

use super::*;
use cgmath::Vector2;

#[derive(Debug, Clone)]
pub struct OrbitBody {
    pub center: Vector2<f64>,
    pub radius: f64,
}
