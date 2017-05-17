
use std;

enum

trait EasingFn {
    fn apply(delta: f64, duration: f64, transition: (f64, f64));
}

#[derive(Clone, Debug)]
struct Data {
    /// a time unit, ie time::precise_time_s()
    start: f64,
    /// a time unit consistent with #start
    duration: f64,
    transitions: Vec<(f64, f64)>,
    easer: Box<EasingFn>,
}

fn diff(transition: (f64, f64)) -> f64 {
    transition.1 - transition.0
}

impl Data {
    fn out_of_bound_value_at(&self, time: f64) -> Option<Vec<f64>> {
        let delta = time - self.start;
        if delta <= 0. {
            return Some(self.transitions.iter().map(|t| t.0).collect());
        }
        if delta >= self.duration {
            return Some(self.transitions.iter().map(|t| t.1).collect());
        }
        None
    }

    pub fn value_at(&self, time: f64) -> Vec<f64> {
        if let Some(vals) = self.out_of_bound_value_at(time) {
            return vals;
        }
        let delta = time - self.start;
        self.transitions.iter()
            .map(|transition| self.easer.apply(delta, self.duration, transition))
            .collect()
    }
}

// impl TtData {
//     // start time
//     fn t(&self) -> f64 { self.start }
//     // change in value
//     fn diff(&self) -> f64 { self.to - self.from }
//     // start value
//     fn b(&self) -> f64 { self.from }
//     // duration
//     fn d(&self) -> f64 { self.duration }
// }

#[derive(Clone, Debug)]
pub struct LinearTransform {
    data: Data
}



impl LinearTransform {
    pub fn new(start: f64, transitions: Vec<(f64, f64)>, duration: f64) -> LinearTransform {
        LinearTransform {
            data: Data {
                start,
                duration,
                transitions,
            }
        }
    }

    pub fn value_at(&self, time: f64) -> Vec<f64> {
        if let Some(vals) = self.data.out_of_bound_value_at(time) {
            return vals;
        }
        let delta = time - self.data.start;
        self.data.transitions.iter()
            .map(|t| {
                // actual easing fn
                diff(*t) * delta / self.data.duration + t.0
            })
            .collect()
    }
}

#[cfg(test)]
mod ease_test {
    use super::*;
    use time;

    const SMALL_ENOUGH: f64 = 0.0000000001;
    const TEST_FROM: f64 = 0.1;
    const TEST_TO: f64 = 12.5;
    const TEST_DURATION: f64 = 0.333;

    #[test]
    fn linear() {
        let t = LinearTransform::new(time::precise_time_s(), vec!((TEST_FROM, TEST_TO)), TEST_DURATION);
        let start = t.data.start;

        assert!((t.value_at(start - TEST_DURATION * 10.)[0] - TEST_FROM).abs() < SMALL_ENOUGH,
            "Before start clamps at #from");

        assert!((t.value_at(start)[0] - TEST_FROM).abs() < SMALL_ENOUGH);
        assert!(t.value_at(start + TEST_DURATION * 0.7)[0] > TEST_FROM);
        assert!(t.value_at(start + TEST_DURATION * 0.7)[0] < TEST_TO);
        assert!((t.value_at(start + TEST_DURATION)[0] - TEST_TO).abs() < SMALL_ENOUGH);

        assert!((t.value_at(start + TEST_DURATION * 10.)[0] - TEST_TO).abs() < SMALL_ENOUGH,
            "After start clamps at #to");
    }
}
