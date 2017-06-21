use single_value_channel;
use std::thread;
use std::time::Duration;
use input::*;
use state::*;
use orbitcurve::OrbitCurve;
use std::sync::mpsc;
use rayon::prelude::*;
use uuid::Uuid;
use compute::compute_state;

pub struct Seer {
    pub projection: single_value_channel::Receiver<Vec<OrbitCurve>>,
    pub main_deltas: mpsc::Sender<f64>,
    pub min_plot_distance: f64,
}

pub const SEER_COMPUTE_DELTA: f64 = 0.001;
pub const SEER_MAX_PLOTS: usize = 50_000;
pub const SEER_FAULT_TOLERANCE: f64 = 0.5;

impl Seer {
    /// The lower the min plot distance the better the curve approximations
    /// with higher load on the GPU, these values are an attempt to optimise
    /// both concerns at different view levels
    pub fn min_plot_distance_at_zoom(zoom: f32) -> f64 {
        if zoom >= 8.5 { 0.27 }
        else if zoom >= 4.5 { 0.18 }
        else if zoom >= 2.5 { 0.15 }
        else if zoom >= 1.5 { 0.1 }
        else { 0.05 }
    }

    // needs &mut as it uses svsc#latest
    #[cfg_attr(feature = "cargo-clippy", allow(wrong_self_convention))]
    pub fn is_approx_as_good_as(&mut self, other: &mut Seer) -> bool {
        let plots = self.projection.latest().get(0)
            .map(|c| c.plots.len())
            .unwrap_or(0) as f64 * self.min_plot_distance;
        let other_plots = other.projection.latest().get(0)
            .map(|c| c.plots.len())
            .unwrap_or(0) as f64 * other.min_plot_distance;

        plots >= other_plots * 0.99
    }

    pub fn new(initial_state: State, tasks: Tasks) -> Seer {
        let (tx, main_deltas_receiver) = mpsc::channel();
        let (projection_get, projection) = single_value_channel::channel_starting_with(Vec::new());

        let zoom = tasks.zoom.as_ref().map(|z| z.zoom_destination()).unwrap_or(initial_state.zoom);
        let min_plot_distance = Seer::min_plot_distance_at_zoom(zoom);

        thread::spawn(move|| {
            let mut tasks = tasks.world_affecting();
            let mut plots = 0;
            let mut state = initial_state;
            let mut main_deltas_ahead = 0.0;

            let mut filtering = false;
            let (tx, filtered_updates) = mpsc::channel();

            state.drawables.orbit_curves.clear();
            for body in &state.drawables.orbit_bodies {
                let mut curve = OrbitCurve::new();
                curve.plots.push(body.center);
                state.drawables.orbit_curves.push(curve);
            }

            let me = Uuid::new_v4();
            loop {
                // consider main loop computed deltas and adjust
                while let Ok(delta) = main_deltas_receiver.try_recv() {
                    main_deltas_ahead += delta;
                }
                let outdated_plots = (main_deltas_ahead / SEER_COMPUTE_DELTA).floor();
                if outdated_plots > 0.0 {
                    main_deltas_ahead -= outdated_plots * SEER_COMPUTE_DELTA;
                    if plots > outdated_plots as usize {
                        plots -= outdated_plots as usize;
                    }
                    else {
                        plots = 0;
                    }
                    for curve in &mut state.drawables.orbit_curves {
                        curve.remove_oldest_plots(outdated_plots as usize);
                    }
                }

                if plots >= SEER_MAX_PLOTS {
                    if projection.has_no_receiver() {
                        break; // dead getter, we've been forgotten
                    }
                    thread::sleep(Duration::from_millis((SEER_COMPUTE_DELTA * 500.0).round() as u64));
                    continue;
                }

                compute_state(&mut state, &mut tasks, SEER_COMPUTE_DELTA);
                for (idx, curve) in state.drawables.orbit_curves.iter_mut().enumerate() {
                    let body = &state.drawables.orbit_bodies[idx];
                    curve.plots.push(body.center);
                }
                plots += 1;

                if !filtering {
                    let curves = state.drawables.orbit_curves.clone();
                    let sender = tx.clone();
                    thread::spawn(move|| {
                        // filtering curves is quite intensive, so use another thread
                        let curves_for_render = curves.par_iter()
                            .map(|c| c.with_minimum_plot_distance(min_plot_distance))
                            .collect();

                        if sender.send(curves_for_render).is_err() {
                            // err => seer is forgotten, not interesting
                        }
                    });
                    filtering = true;
                }

                if let Ok(curves) = filtered_updates.try_recv() {
                    if projection.update(curves).is_err() {
                        break; // dead getter, we've been forgotten
                    }
                    filtering = false;
                }
            }
            trace!("Seer {} forgetten", me);
        });

        Seer {
            projection: projection_get,
            main_deltas: tx,
            min_plot_distance,
        }
    }
}
