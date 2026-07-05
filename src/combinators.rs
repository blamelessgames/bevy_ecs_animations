use std::{
    marker::PhantomData,
    ops::{Add, Mul, Sub},
};

use bevy_math::{Curve, FloatExt, StableInterpolate, curve::Interval};

/// just a helper for type-erased curve storage in bevy
pub type BoxedCurve<T> = Box<dyn Curve<T> + Send + Sync + 'static>;

pub struct Constant<T> {
    low: f32,
    high: f32,
    value: T,
}

/// a very flat curve
pub const fn constant<T: Clone>(low: f32, high: f32, value: T) -> Constant<T> {
    Constant { low, high, value }
}

impl<T: Clone> Curve<T> for Constant<T> {
    fn domain(&self) -> Interval {
        Interval::new(self.low, self.high).unwrap()
    }

    fn sample_unchecked(&self, _: f32) -> T {
        self.value.clone()
    }
}

// follows one curve with the next. not sure if we want to
// adjust the second's domain to the end of the first, i.e. require it
// to start at zero, or if they are responsible for time and we just hold
#[derive(Clone, Copy)]
pub struct Seq<T, C1, C2> {
    first_curve: C1,
    second_curve: C2,
    _marker: PhantomData<fn() -> T>,
}

/// follows one curve with the next, respecting underlying domains. if there is a gap between
/// domains the end of the first curve is held to cover it
pub const fn seq<T, C1: Curve<T>, C2: Curve<T>>(first: C1, second: C2) -> Seq<T, C1, C2> {
    Seq {
        first_curve: first,
        second_curve: second,
        _marker: PhantomData,
    }
}

impl<T, C1: Curve<T>, C2: Curve<T>> Curve<T> for Seq<T, C1, C2> {
    fn domain(&self) -> Interval {
        debug_assert!(self.first_curve.domain().end() <= self.second_curve.domain().start());
        Interval::new(
            self.first_curve.domain().start(),
            self.second_curve.domain().end(),
        )
        .unwrap()
    }

    fn sample_unchecked(&self, t: f32) -> T {
        debug_assert!(self.first_curve.domain().end() <= self.second_curve.domain().start());
        if self.first_curve.domain().contains(t) {
            self.first_curve.sample_unchecked(t)
        } else if self.second_curve.domain().contains(t) {
            self.second_curve.sample_unchecked(t)
        } else {
            self.first_curve
                .sample_unchecked(self.first_curve.domain().end())
        }
    }
}

pub struct Zip<T1, T2, C1, C2> {
    first: C1,
    second: C2,
    _marker1: PhantomData<fn() -> T1>,
    _marker2: PhantomData<fn() -> T2>,
}

/// combines two curves into a 2-tuple. curves are clamped to
/// their domains, meaning they will hold their edges to make
/// up gaps. the domain of the resulting curve is the union of
/// the domains of the contained curves
pub const fn zip<T1, T2, C1: Curve<T1>, C2: Curve<T2>>(
    first: C1,
    second: C2,
) -> Zip<T1, T2, C1, C2> {
    Zip {
        first,
        second,
        _marker1: PhantomData,
        _marker2: PhantomData,
    }
}

impl<T1, T2, C1: Curve<T1>, C2: Curve<T2>> Curve<(T1, T2)> for Zip<T1, T2, C1, C2> {
    fn domain(&self) -> Interval {
        Interval::new(
            self.first
                .domain()
                .start()
                .min(self.second.domain().start()),
            self.first.domain().end().max(self.second.domain().end()),
        )
        .unwrap()
    }

    fn sample_unchecked(&self, t: f32) -> (T1, T2) {
        (self.first.sample_clamped(t), self.second.sample_clamped(t))
    }
}

pub struct Pipe<T, C1, C2> {
    source: C1,
    sink: C2,
    _marker_out: PhantomData<fn() -> T>,
}

/// sends the output of source to the input of sink
pub const fn pipe<T, C1, C2>(source: C1, sink: C2) -> Pipe<T, C1, C2>
where
    C1: Curve<f32>,
    C2: Curve<T>,
{
    Pipe {
        source,
        sink,
        _marker_out: PhantomData,
    }
}

impl<T, C1, C2> Curve<T> for Pipe<T, C1, C2>
where
    C1: Curve<f32>,
    C2: Curve<T>,
{
    fn domain(&self) -> Interval {
        self.source.domain()
    }

    fn sample_unchecked(&self, t: f32) -> T {
        self.sink.sample_clamped(self.source.sample_unchecked(t))
    }
}

pub struct Interpolate<T, C1, C2, C3> {
    first: C1,
    second: C2,
    t: C3,
    _marker: PhantomData<fn() -> T>,
}

/// interpolate between first and second using the result from t. all three curves must have
/// the same domain. the interpolation is through implementations of Bevy's `StableInterpolate`
/// trait, so it should be well-behaved for all supported types
pub const fn interpolate<T, C1, C2, C3>(first: C1, second: C2, t: C3) -> Interpolate<T, C1, C2, C3>
where
    C1: Curve<T>,
    C2: Curve<T>,
    C3: Curve<f32>,
    T: StableInterpolate,
{
    Interpolate {
        first,
        second,
        t,
        _marker: PhantomData,
    }
}

impl<T, C1, C2, C3> Curve<T> for Interpolate<T, C1, C2, C3>
where
    C1: Curve<T>,
    C2: Curve<T>,
    C3: Curve<f32>,
    T: StableInterpolate,
{
    fn domain(&self) -> Interval {
        debug_assert_eq!(self.first.domain(), self.second.domain());
        debug_assert_eq!(self.first.domain(), self.t.domain());
        self.first.domain()
    }

    fn sample_unchecked(&self, t: f32) -> T {
        self.first
            .sample_unchecked(t)
            .interpolate_stable(&self.second.sample_unchecked(t), self.t.sample_unchecked(t))
    }
}

pub struct Map<T, O, C, F> {
    curve: C,
    mapper: F,
    _marker_in: PhantomData<fn() -> T>,
    _marker_out: PhantomData<fn() -> O>,
}

/// runs the given function over each t value. the most general combinator of all
pub const fn map<T, O, C, F>(curve: C, mapper: F) -> Map<T, O, C, F>
where
    C: Curve<T>,
    F: Fn(T) -> O,
{
    Map {
        curve,
        mapper,
        _marker_in: PhantomData,
        _marker_out: PhantomData,
    }
}

impl<T, O, C, F> Curve<O> for Map<T, O, C, F>
where
    C: Curve<T>,
    F: Fn(T) -> O,
{
    fn domain(&self) -> Interval {
        self.curve.domain()
    }

    fn sample_unchecked(&self, t: f32) -> O {
        (self.mapper)(self.curve.sample_unchecked(t))
    }
}

/// delays the underlying curve by `delay` seconds
pub struct Delay<T, C> {
    delay: f32,
    curve: C,
    _marker: PhantomData<fn() -> T>,
}

/// pushes the domain of the contained curve forward by `delay`, in other
/// words a delay of 1.0 applied to a curve with a domain of 0.0..1.0 will
/// produce a curve with a domain of 0.0..2.0, and the contained curve will
/// see its original domain once the timeline hits 1.0
pub const fn delay<T, C: Curve<T>>(delay: f32, curve: C) -> Delay<T, C> {
    Delay {
        delay,
        curve,
        _marker: PhantomData,
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

pub struct ScaledOutput<T, C, O> {
    low: T,
    high: T,
    curve: C,
    _marker_out: PhantomData<fn() -> O>,
}

/// scales unit output. if you need something with more finesse, [map] or [fn_curve] it
pub const fn scaled_output<T, C, O>(low: T, high: T, curve: C) -> ScaledOutput<T, C, O>
where
    C: Curve<O>,
    T: Sub<Output = T> + Clone,
    O: Add<T, Output = O> + Mul<T, Output = O>,
{
    ScaledOutput {
        low,
        high,
        curve,
        _marker_out: PhantomData,
    }
}

impl<T, C, O> Curve<O> for ScaledOutput<T, C, O>
where
    C: Curve<O>,
    T: Sub<Output = T> + Clone,
    O: Add<T, Output = O> + Mul<T, Output = O>,
{
    fn domain(&self) -> Interval {
        self.curve.domain()
    }

    fn sample_unchecked(&self, t: f32) -> O {
        self.curve.sample_unchecked(t) * (self.high.clone() - self.low.clone()) + self.low.clone()
    }
}

/// scales the underlying curve so its domain is exposed
/// as low->high instead. like if you just want to add
/// an easing function for a stretch in a sequence
pub struct ScaledDomain<T, C> {
    low: f32,
    high: f32,
    curve: C,
    _marker: PhantomData<fn() -> T>,
}

/// scales the underlying curve so its domain is exposed
/// as low->high instead.
pub const fn scaled_domain<T, C: Curve<T>>(low: f32, high: f32, curve: C) -> ScaledDomain<T, C> {
    debug_assert!(low < high && low >= 0.0);
    ScaledDomain {
        low,
        high,
        curve,
        _marker: PhantomData,
    }
}

impl<T, C: Curve<T>> From<(f32, f32, C)> for ScaledDomain<T, C> {
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

impl<T, C: Curve<T>> Curve<T> for ScaledDomain<T, C> {
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

pub struct FnCurve<Func, O> {
    start: f32,
    end: f32,
    function: Func,
    _marker: PhantomData<fn() -> O>,
}

/// exposes the given function with the given domain
pub const fn fn_curve<Func, O>(start: f32, end: f32, function: Func) -> FnCurve<Func, O>
where
    Func: Fn(f32) -> O,
{
    FnCurve {
        start,
        end,
        function,
        _marker: PhantomData,
    }
}

impl<Func, O> Curve<O> for FnCurve<Func, O>
where
    Func: Fn(f32) -> O,
{
    fn domain(&self) -> Interval {
        Interval::new(self.start, self.end).unwrap()
    }

    fn sample_unchecked(&self, t: f32) -> O {
        (self.function)(t)
    }
}

pub struct Signal<const N: usize, T> {
    low: f32,
    high: f32,
    triggers: [(f32, T); N],
    tolerance: f32,
}

/// returns the configured instance when the t value is within tolerance of the setting
/// that is to say `signal(`
pub fn signal<const N: usize, T>(
    low: f32,
    high: f32,
    triggers: [(f32, T); N],
    // this is a bad answer. might actually need a thread local here
    // and some coordination with the tick mechanism to pass the actual
    // dt in
    tolerance: f32,
) -> Signal<N, T>
where
    T: Clone,
{
    Signal {
        low,
        high,
        triggers,
        tolerance,
    }
}

impl<const N: usize, T> Curve<Option<T>> for Signal<N, T>
where
    T: Clone,
{
    fn domain(&self) -> Interval {
        Interval::new(self.low, self.high).unwrap()
    }

    fn sample_unchecked(&self, t: f32) -> Option<T> {
        // this is the only const approach i can think of...
        self.triggers
            .iter()
            .find(|(at, _)| *at >= t && *at <= t + self.tolerance)
            .map(|(_, signal)| signal.clone())
    }
}

#[test]
fn test_combinators() {
    // i'm not sure if any of this is useful tbh
    use bevy_math::prelude::*;
    const fn crazy() -> impl Curve<(Vec3, Vec4)> {
        zip(
            scaled_output(
                22.0,
                100.0,
                map(
                    zip(
                        scaled_domain(0.0, 2.0, EaseFunction::BackIn),
                        delay(1.0, EaseFunction::BackOut),
                    ),
                    |(x, y)| vec3(x, y, 0.4),
                ),
            ),
            map(
                zip(
                    scaled_domain(
                        0.0,
                        2.0,
                        zip(
                            EaseFunction::CircularIn,
                            scaled_output(
                                360.0,
                                -360.0,
                                // this is probably just multiplying them but it works!
                                pipe(EaseFunction::QuadraticIn, EaseFunction::CubicInOut),
                            ),
                        ),
                    ),
                    // this curve will hold at all zeroes for the first
                    // second, then join in on the fun
                    scaled_domain(
                        1.0,
                        2.0,
                        zip(EaseFunction::QuadraticInOut, EaseFunction::SmoothStep),
                    ),
                ),
                |((x, y), (z, w))| vec4(x, y, z, w),
            ),
        )
    }

    let c = crazy();
    assert_eq!(c.domain().start(), 0.0);
    assert_eq!(c.domain().end(), 2.0);
    // 60 frames a second oughta be enough for anyone
    for t in c.domain().spaced_points(120).unwrap() {
        println!("{:?}", c.sample_unchecked(t));
    }
}
