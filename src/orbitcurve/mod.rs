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
        // capacity optimisation guess, filtering should generally be reducing the plots
        // by an order of magnitude
        let mut plots = Vec::with_capacity(self.plots.len() / 10);
        plots.push(self.plots[0]);
        let mut last_plot = plots[0];
        let mut last_plot_idx = 0;
        let mut last_gap = 1;
        let mut counter = 1;
        let mut expect_next_max_distance2 = 0f64;

        loop {
            // Plots should be ~equi-distant number of plots inside min should be similarly constant.
            // After checking each subsequent index we have a number of plots inside the min N.
            // Next simply skip forward to N-1, if it is also inside the min we can assume 1..N-2 also
            // were (potentially dangerous with very chaotic data). if otherwise revert to checking
            // every index again
            // benchmarks 10x faster than simple iteration
            if last_gap > 2 && last_plot_idx + last_gap < self.plots.len() {
                // try to skip by the same gap as viewed last time
                let next = self.plots[last_plot_idx + last_gap];
                let next_distance2 = last_plot.distance2(next);
                if next_distance2 >= min_distance2 && next_distance2 <= expect_next_max_distance2 {
                    // gap satisfied expectations
                    plots.push(next);
                    last_plot = next;
                    last_plot_idx = last_plot_idx + last_gap;
                    continue;
                }
                else {
                    // didn't work, revert to +1 index checking
                    last_gap = 1;
                }
            }

            match self.plots.get(last_plot_idx + counter) {
                None => break,
                Some(plot) => {
                    let distance2 = last_plot.distance2(*plot);
                    if distance2 >= min_distance2 {
                        plots.push(*plot);
                        last_plot = *plot;
                        last_plot_idx += counter;
                        last_gap = counter;
                        expect_next_max_distance2 = distance2 * 1.1;
                        counter = 1;
                    }
                    else {
                        counter += 1;
                    }
                }
            }
        }

        plots.shrink_to_fit();
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

    #[test]
    fn with_minimum_plot_distance() {
        let mut curve = OrbitCurve::new();
        for i in 0..900 {
            curve.plots.push((i as f64, i as f64).into());
        }

        let filtered = curve.with_minimum_plot_distance(2.9); // ie bigger than 2 * sqrt(2)

        assert_eq!(filtered.plots.len(), 300);
        assert_eq!(filtered.plots[0].x, 0.0);
        assert_eq!(filtered.plots[1].x, 3.0);
        assert_eq!(filtered.plots[2].x, 6.0);
        assert_eq!(filtered.plots[3].x, 9.0);
    }

    #[cfg(feature = "bench")]
    use test::Bencher;

    #[cfg(feature = "bench")]
    #[bench]
    fn bench_with_minimum_plot_distance(b: &mut Bencher) {
        let mut curve = OrbitCurve::new();
        let points = 50_000;
        // plot quad bezier 2(1-t)(P1 - P0) + 2t(P2 - P1)
        let p0 = Vector2::new(0.0, 0.0);
        let p1 = Vector2::new(30.0, 30.0);
        let p2 = Vector2::new(60.0, 0.0);

        for i in 1..(points+1) {
            let t = i as f64 / points as f64;
            let plot = 2.0 * (1.0 - t) * (p1 - p0) + 2. * t * (p2 - p1);
            curve.plots.push(plot);
        }
        assert_eq!(curve.plots.len(), points);
        assert_eq!(curve.with_minimum_plot_distance(0.1).plots.len(), 1191);

        b.iter(|| curve.with_minimum_plot_distance(0.1));
    }
}
