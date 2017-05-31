pub mod render;

use cgmath::*;

#[derive(Debug, Clone)]
pub struct OrbitCurve {
    plots: Vec<Vector2<f64>>,
    angle: Rad<f64>,
}

impl OrbitCurve {
    pub fn new() -> OrbitCurve {
        OrbitCurve { plots: Vec::new(), angle: Rad::zero() }
    }

    pub fn add_plot(&mut self, plot: Vector2<f64>) -> bool {
        if self.angle >= Rad::full_turn() {
            return false;
        }

        if self.plots.len() > 1 {
            self.angle += self.plots[self.plots.len()-1].angle(plot);
        }
        self.plots.push(plot);
        true
    }

    pub fn mean_plot(&self) -> Vector2<f64> {
        self.plots.iter().sum::<Vector2<f64>>() / self.plots.len() as f64
    }

    pub fn is_drawable(&self) -> bool {
        self.plots.len() > 3 && self.mean_plot().distance(self.plots[0]) > 0.1
    }
}

#[cfg(test)]
mod orbitcurve_compute {
    use super::*;

    #[test]
    fn mean_plot() {
        let curve = OrbitCurve {
            plots: vec!((1.0, 0.0).into(), (1.0, 2.0).into(), (1.0, 4.0).into())
        };
        assert_eq!(curve.mean_plot(), (1.0, 2.0).into());
    }
}
