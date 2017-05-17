
#[derive(Clone, Copy, Debug)]
struct TtData {
    /// seconds, ie time::precise_time_s()
    start: f64,
    /// seconds
    duration: f64,
    from: f64,
    to: f64,
}

impl TtData {
    // start time
    fn t(&self) -> f64 { self.start }
    // change in value
    fn diff(&self) -> f64 { self.to - self.from }
    // start value
    fn b(&self) -> f64 { self.from }
    // duration
    fn d(&self) -> f64 { self.duration }
}

#[derive(Clone, Copy, Debug)]
pub struct LinearTransform {
    data: TtData
}

impl LinearTransform {
    pub fn new(start: f64, from: f64, to: f64, duration: f64) -> LinearTransform {
        LinearTransform {
            data: TtData {
                start: start,
                duration,
                from,
                to,
            }
        }
    }

    pub fn value_at(&self, time: f64) -> f64 {
        let t = &self.data;
        let delta = time - t.start;
        if delta <= 0. { return t.from; }
        if delta > t.duration { return t.to; }
        t.diff() * delta / t.duration + t.from
    }
}

#[cfg(test)]
mod smooth_test {
    use super::*;
    use time;

    const SMALL_ENOUGH: f64 = 0.0000000001;
    const TEST_FROM: f64 = 0.1;
    const TEST_TO: f64 = 12.5;
    const TEST_DURATION: f64 = 0.333;

    #[test]
    fn linear() {
        let t = LinearTransform::new(time::precise_time_s(), TEST_FROM, TEST_TO, TEST_DURATION);
        let start = t.data.start;

        assert!((t.value_at(start - TEST_DURATION * 10.) - TEST_FROM).abs() < SMALL_ENOUGH,
            "Before start clamps at #from");

        assert!((t.value_at(start) - TEST_FROM).abs() < SMALL_ENOUGH);
        assert!(t.value_at(start + TEST_DURATION * 0.7) > TEST_FROM);
        assert!(t.value_at(start + TEST_DURATION * 0.7) < TEST_TO);
        assert!((t.value_at(start + TEST_DURATION) - TEST_TO).abs() < SMALL_ENOUGH);

        assert!((t.value_at(start + TEST_DURATION * 10.) - TEST_TO).abs() < SMALL_ENOUGH,
            "After start clamps at #to");
    }
}
