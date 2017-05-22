
#[derive(Clone, Debug)]
pub struct Easer<T> {
    /// a floating point time unit, ie time::precise_time_s()
    pub start: T,
    /// a time unit consistent with #start
    pub duration: T,
    /// list of (to, from) transitions
    pub transitions: Vec<(T, T)>,
    /// easing function fn(t,b,c,d) -> float
    easing: fn(T, T, T, T) -> T,
}

use num_traits::{Float, Zero};

fn linear_easing<F: Float>(t: F, b: F, c: F, d: F) -> F {
    c * t / d + b
}

impl<F: Float + Zero> Easer<F> {
    pub fn using(easing: fn(F, F, F, F) -> F) -> Easer<F> {
        Easer {
            start: F::zero(),
            duration: F::zero(),
            transitions: Vec::new(),
            easing: easing,
        }
    }

    pub fn linear() -> Easer<F> {
        Easer::using(linear_easing)
    }

    pub fn start<T: Into<F>>(mut self, start: T) -> Easer<F> {
        self.start = start.into();
        self
    }

    pub fn duration<T: Into<F>>(mut self, duration: T) -> Easer<F> {
        self.duration = duration.into();
        self
    }

    pub fn add_transition<T: Into<F>, V: Into<F>>(mut self, from: T, to: V) -> Easer<F> {
        self.transitions.push((from.into(), to.into()));
        self
    }

    fn out_of_bound_values_at(&self, time: F) -> Option<Vec<F>> {
        let delta = time - self.start;
        if delta <= F::zero() {
            return Some(self.transitions.iter().map(|t| t.0).collect());
        }
        if delta >= self.duration {
            return Some(self.transitions.iter().map(|t| t.1).collect());
        }
        None
    }

    pub fn values_at<T: Into<F>>(&self, time: T) -> Vec<F> {
        let time: F = time.into();
        if let Some(vals) = self.out_of_bound_values_at(time) {
            return vals;
        }
        let t = time - self.start;
        let d = self.duration;
        self.transitions.iter()
            .map(|transition| {
                let b = transition.0;
                let c = transition.1 - transition.0;
                (self.easing)(t, b, c, d)
            })
            .collect()
    }

    pub fn has_finished<T: Into<F>>(&self, time: T) -> bool {
        let time: F = time.into();
        time > self.start + self.duration
    }
}

#[cfg(test)]
mod ease_test {
    use super::*;
    use easer::functions::*;

    type F = f32;
    const SMALL_ENOUGH: F = 0.000000001;
    const TEST_FROM: F = 0.1;
    const TEST_TO: F = 12.5;
    const TEST_START: F = 1234.3456;
    const TEST_DURATION: F = 0.333;

    macro_rules! check {
        ($easer:ident; $type:ty) => {
            let start = $easer.start;
            let duration = $easer.duration;
            let from_val = $easer.transitions[0].0;
            let to_val = $easer.transitions[0].1;
            let cast_small_val = SMALL_ENOUGH as $type;

            assert!(($easer.values_at(start - duration * 10.)[0] - from_val).abs() < cast_small_val,
                "Before start clamps at #from");
            assert!(($easer.values_at(start + duration * 10.)[0] - to_val).abs() < cast_small_val,
                "After start clamps at #to");

            assert!(($easer.values_at(start)[0] - from_val).abs() < cast_small_val,
                format!("Got: {}", $easer.values_at(start)[0]));
            assert!(($easer.values_at(start + duration)[0] - to_val).abs() < cast_small_val,
                format!("Got: {}", $easer.values_at(start + duration)[0]));

            // not technically required, but true for test cases
            assert!($easer.values_at(start + duration * 0.5)[0] > from_val,
                "Half way is before start");
            assert!($easer.values_at(start + duration * 0.5)[0] < to_val,
                "Half way is after end");
        }
    }

    #[test]
    fn linear_32() {
        let easer: Easer<f32> = Easer::linear()
            .duration(TEST_DURATION)
            .start(TEST_START)
            .add_transition(TEST_FROM, TEST_TO);

        println!("32: {:?}", easer.values_at(1234.4));
        check!(easer; f32);
    }

    #[test]
    fn linear_64() {
        let easer = Easer::<f64>::linear()
            .duration(TEST_DURATION)
            .start(TEST_START)
            .add_transition(TEST_FROM, TEST_TO);

        println!("64: {:?}", easer.values_at(1234.4));
        check!(easer; f64);
    }

    #[test]
    fn generic_32() {
        let cubic_in = Easer::<f32>::using(Cubic::ease_in)
            .duration(TEST_DURATION)
            .start(TEST_START)
            .add_transition(TEST_FROM, TEST_TO);

        let val = cubic_in.values_at(TEST_START + TEST_DURATION / 2.0);
        println!("32: {:?}", val);

        check!(cubic_in; f32);
    }

    #[test]
    fn generic_64() {
        let cubic_in = Easer::<f64>::using(Cubic::ease_in)
            .duration(TEST_DURATION)
            .start(TEST_START)
            .add_transition(TEST_FROM, TEST_TO);

        let val = cubic_in.values_at(TEST_START + TEST_DURATION / 2.0);
        println!("64: {:?}", val);

        check!(cubic_in; f64);
    }
}
