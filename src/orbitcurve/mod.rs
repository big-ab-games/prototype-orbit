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

    pub fn is_drawable(&self) -> bool {
        self.plots.len() > 3
    }

    pub fn with_minimum_plot_distance(&self, min_distance: f64) -> OrbitCurve {
        // Reduce plots to a min distance apart from one another, to reduce render load
        let min_distance2 = min_distance * min_distance;
        let mut plots = Vec::new();
        plots.push(self.plots[0]);
        let mut last_plot = plots[0];
        for plot in self.plots.iter() {
            if last_plot.distance2(*plot) > min_distance2 {
                plots.push(*plot);
                last_plot = *plot;
            }
        }
        OrbitCurve { plots, opacity: self.opacity }
    }
}

#[cfg(test)]
mod orbitcurve_compute {
    use super::*;

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
