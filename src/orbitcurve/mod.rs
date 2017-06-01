pub mod render;

use cgmath::*;

#[derive(Debug, Clone)]
pub struct OrbitCurve {
    pub plots: Vec<Vector2<f64>>,
    pub opacity: f32,
}

impl OrbitCurve {
    pub fn new() -> OrbitCurve {
        OrbitCurve { plots: Vec::new(), opacity: 1.0 }
    }

    pub fn remove_oldest_plots(&mut self, n: usize) {
        if n >= self.plots.len() {
            self.plots.clear();
        }
        else {
            for _ in 0..n {
                self.plots.remove(0);
            }
        }
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
        let mut curve = OrbitCurve::new();
        curve.plots.push((1.0, 0.0).into());
        curve.plots.push((1.0, 2.0).into());
        curve.plots.push((1.0, 4.0).into());

        assert_eq!(curve.mean_plot(), (1.0, 2.0).into());
    }

    #[test]
    fn remove_oldest_plots() {
        let mut curve = OrbitCurve::new();
        curve.plots.push((1.0, 0.0).into());
        curve.plots.push((2.0, 0.0).into());
        curve.plots.push((3.0, 0.0).into());
        curve.plots.push((4.0, 0.0).into());

        curve.remove_oldest_plots(0);
        assert_eq!(curve.plots.len(), 4);
        assert_eq!(curve.plots[0].x, 1.0);

        curve.remove_oldest_plots(1);
        assert_eq!(curve.plots.len(), 3);
        assert_eq!(curve.plots[0].x, 2.0);

        curve.remove_oldest_plots(2);
        assert_eq!(curve.plots.len(), 1);
        assert_eq!(curve.plots[0].x, 4.0);

        curve.remove_oldest_plots(500);
        assert_eq!(curve.plots.len(), 0);
    }
}
