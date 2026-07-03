use std::marker::PhantomData;

use bevy_math::{Curve, FloatExt, curve::Interval};

// follows one curve with the next. not sure if we want to
// adjust the second's domain to the end of the first, i.e. require it
// to start at zero, or if they are responsible for time and we just hold
#[derive(Clone, Copy)]
pub struct Seq<T, C1, C2> {
    first_curve: C1,
    second_curve: C2,
    _marker: PhantomData<T>,
}

impl<T, C1: Curve<T>, C2: Curve<T>> Seq<T, C1, C2> {
    pub const fn new(first: C1, second: C2) -> Self {
        Seq {
            first_curve: first,
            second_curve: second,
            _marker: PhantomData,
        }
    }
}

impl<T, C1: Curve<T>, C2: Curve<T>> Curve<T> for Seq<T, C1, C2> {
    fn domain(&self) -> Interval {
        debug_assert!(self.first_curve.domain().end() <= self.second_curve.domain().start());
        Interval::new(
            self.first_curve.domain().start(),
            self.second_curve.domain().end(),
        )
        .expect("come on man")
    }

    fn sample_unchecked(&self, t: f32) -> T {
        debug_assert!(self.first_curve.domain().end() <= self.second_curve.domain().start());
        if self.first_curve.domain().contains(t) {
            self.first_curve.sample_unchecked(t)
        } else if self.second_curve.domain().contains(t) {
            self.second_curve.sample_unchecked(t)
        } else {
            // for now just hold the first end. this isn't really meant to be foolproof yet
            self.first_curve
                .sample_unchecked(self.first_curve.domain().end())
        }
    }
}

/// delays the underlying curve by `delay` seconds
pub struct Delay<T, C> {
    delay: f32,
    curve: C,
    _marker: PhantomData<T>,
}

impl<T, C: Curve<T>> Delay<T, C> {
    pub const fn new(delay: f32, curve: C) -> Self {
        Delay {
            delay,
            curve,
            _marker: PhantomData,
        }
    }
}

impl<T, C: Curve<T>> From<(f32, C)> for Delay<T, C> {
    fn from((delay, curve): (f32, C)) -> Self {
        debug_assert!(delay >= 0.0);
        Self {
            delay,
            curve,
            _marker: PhantomData,
        }
    }
}

impl<T, C: Curve<T>> Curve<T> for Delay<T, C> {
    fn domain(&self) -> Interval {
        let parent = self.curve.domain();
        Interval::new(parent.start() + self.delay, parent.end() + self.delay).unwrap()
    }

    fn sample_unchecked(&self, t: f32) -> T {
        self.curve.sample_unchecked(t - self.delay)
    }
}

// scales the output. only works when the output is
// f32, if i need more i'll bring in num traits
pub struct ScaledOutput<T, C> {
    low: T,
    high: T,
    curve: C,
}

impl<T, C: Curve<T>> ScaledOutput<T, C> {
    pub const fn new(low: T, high: T, curve: C) -> Self {
        ScaledOutput { low, high, curve }
    }
}

impl<C: Curve<f32>> From<(f32, f32, C)> for ScaledOutput<f32, C> {
    fn from((low, high, curve): (f32, f32, C)) -> Self {
        Self { low, high, curve }
    }
}

impl<C: Curve<f32>> Curve<f32> for ScaledOutput<f32, C> {
    fn domain(&self) -> Interval {
        self.curve.domain()
    }

    fn sample_unchecked(&self, t: f32) -> f32 {
        self.curve.sample_unchecked(t) * (self.high - self.low) + self.low
    }
}

/// scales the underlying curve so its domain is exposed
/// as low->high instead. like if you just want to add
/// an easing function for a stretch in a sequence
pub struct ScaledTime<T, C> {
    low: f32,
    high: f32,
    curve: C,
    _marker: PhantomData<T>,
}

impl<T, C: Curve<T>> ScaledTime<T, C> {
    pub const fn new(low: f32, high: f32, curve: C) -> Self {
        debug_assert!(low < high && low >= 0.0);
        ScaledTime {
            low,
            high,
            curve,
            _marker: PhantomData,
        }
    }
}

impl<T, C: Curve<T>> From<(f32, f32, C)> for ScaledTime<T, C> {
    fn from((low, high, curve): (f32, f32, C)) -> Self {
        debug_assert!(low < high && low >= 0.0);
        Self {
            low,
            high,
            curve,
            _marker: PhantomData,
        }
    }
}

impl<T, C: Curve<T>> Curve<T> for ScaledTime<T, C> {
    fn domain(&self) -> Interval {
        Interval::new(self.low, self.high).unwrap()
    }

    fn sample_unchecked(&self, t: f32) -> T {
        let t = (t - self.low) / (self.high - self.low);
        let t = self
            .curve
            .domain()
            .start()
            .lerp(self.curve.domain().end(), t);
        self.curve.sample_unchecked(t)
    }
}

pub enum Function {
    Sin,
    Cos,
}

impl Function {
    fn call(&self, t: f32) -> f32 {
        match self {
            Function::Sin => f32::sin(t),
            Function::Cos => f32::cos(t),
        }
    }
}

pub struct FunctionCurve {
    duration: f32,
    low: f32,
    high: f32,
    f: Function,
    magnitude: f32,
}

impl From<(f32, f32, f32, Function, f32)> for FunctionCurve {
    fn from((duration, low, high, f, magnitude): (f32, f32, f32, Function, f32)) -> Self {
        debug_assert!(duration > 0.0 && high > low && magnitude > 0.0);
        Self {
            duration,
            low,
            high,
            f,
            magnitude,
        }
    }
}

impl Curve<f32> for FunctionCurve {
    fn domain(&self) -> Interval {
        Interval::new(0.0, self.duration).unwrap()
    }

    fn sample_unchecked(&self, t: f32) -> f32 {
        self.f
            .call(((t / self.duration) * (self.high - self.low)) + self.low)
            * self.magnitude
    }
}
