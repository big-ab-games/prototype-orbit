
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

macro_rules! implement_easer {
    ($x:ty, using -> $using_name:ident, linear -> $linear_name:ident) => {
        impl Easer<$x> {
            pub fn $using_name(easing: fn($x, $x, $x, $x) -> $x) -> Easer<$x> {
                Easer {
                    start: 0.0,
                    duration: 0.0,
                    transitions: Vec::new(),
                    easing: easing,
                }
            }

            pub fn $linear_name() -> Easer<$x> {
                fn linear_easing(t: $x, b: $x, c: $x, d: $x) -> $x {
                    c * t / d + b
                }
                Easer::$using_name(linear_easing)
            }

            pub fn start<T: Into<$x>>(mut self, start: T) -> Easer<$x> {
                self.start = start.into();
                self
            }

            pub fn duration<T: Into<$x>>(mut self, duration: T) -> Easer<$x> {
                self.duration = duration.into();
                self
            }

            pub fn add_transition<T: Into<$x>, V: Into<$x>>(mut self, from: T, to: V) -> Easer<$x> {
                self.transitions.push((from.into(), to.into()));
                self
            }

            fn out_of_bound_values_at(&self, time: $x) -> Option<Vec<$x>> {
                let delta = time - self.start;
                if delta <= 0. {
                    return Some(self.transitions.iter().map(|t| t.0).collect());
                }
                if delta >= self.duration {
                    return Some(self.transitions.iter().map(|t| t.1).collect());
                }
                None
            }

            pub fn values_at<T: Into<$x>>(&self, time: T) -> Vec<$x> {
                let time: $x = time.into();
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

            pub fn has_finished<T: Into<$x>>(&self, time: T) -> bool {
                let time: $x = time.into();
                time > self.start + self.duration
            }
        }
    }
}

implement_easer!(f64, using -> using, linear -> linear);
implement_easer!(f32, using -> using32, linear -> linear32);

#[cfg(test)]
mod ease_test {
    use super::*;

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
    fn linear() {
        let easer = Easer::linear()
            .duration(TEST_DURATION)
            .start(TEST_START)
            .add_transition(TEST_FROM, TEST_TO);
        check!(easer; f64);
    }

    #[test]
    fn linear_32() {
        let easer = Easer::linear32()
            .duration(TEST_DURATION)
            .start(TEST_START)
            .add_transition(TEST_FROM, TEST_TO);
        check!(easer; f32);
    }

    #[test]
    fn easer_functions_32() {
        use easer::functions::*;

        let circ_in = Easer::using32(Circ::ease_in)
            .duration(TEST_DURATION)
            .start(TEST_START)
            .add_transition(TEST_FROM, TEST_TO);

        let expo_out = Easer::using32(Expo::ease_out)
            .duration(TEST_DURATION)
            .start(TEST_START)
            .add_transition(TEST_FROM, TEST_TO);

        let sin_in_out = Easer::using32(Sine::ease_in_out)
            .duration(TEST_DURATION)
            .start(TEST_START)
            .add_transition(TEST_FROM, TEST_TO);

        check!(circ_in; f32);
        check!(expo_out; f32);
        check!(sin_in_out; f32);
    }
}
