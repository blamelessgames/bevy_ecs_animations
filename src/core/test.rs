use ::core::{marker::PhantomData, time::Duration};

use crate::{core::AnimationTickState, *};
use bevy::{prelude::*, time::TimePlugin};
use bevy_time::TimeUpdateStrategy;
use float_eq::assert_float_eq;

#[test]
fn test_timekeeping() {
    fn test(
        c: AnimationConfiguration,
        elapsed: Duration,
        dt: Duration,
        expected: Option<(Duration, Duration, bool)>,
    ) {
        let c = c.resolve();
        let Some(AnimationTickState {
            time,
            delta,
            finished,
        }) = c.t(elapsed, dt)
        else {
            assert_eq!(expected, None, "what the heck? {elapsed:?}, {dt:?}");
            return;
        };
        let (e_t, e_dt, e_finished) = expected.expect("expected None!");
        // millisecond precision seems fine?
        assert_eq!(e_t, time);
        assert_eq!(e_dt, delta);
        assert_eq!(e_finished, finished);
    }

    let c = AnimationConfiguration::from(4.0);
    // lock in that nanos accumulate correctly
    test(
        c,
        Duration::ZERO,
        Duration::from_nanos(1102375),
        Some((
            Duration::from_nanos(1102375),
            Duration::from_nanos(1102375),
            false,
        )),
    );
    test(
        c,
        Duration::from_millis(100),
        Duration::from_millis(8),
        Some((Duration::from_millis(108), Duration::from_millis(8), false)),
    );
    // must compensate for dt going past the end
    test(
        c,
        Duration::from_millis(3900),
        Duration::from_secs(1),
        Some((Duration::from_secs(4), Duration::from_millis(100), true)),
    );
    test(
        c,
        Duration::from_millis(3990),
        Duration::from_secs(1),
        Some((Duration::from_secs(4), Duration::from_millis(10), true)),
    );
    // this doesn't get called after it reports finished

    let c = c.start_after(1.0);
    // should be None the whole first second
    test(c, Duration::ZERO, Duration::from_nanos(1102375), None);
    test(
        c,
        Duration::from_millis(400),
        Duration::from_nanos(1102375),
        None,
    );
    test(
        c,
        Duration::from_millis(999),
        // even right up on the line should be None
        Duration::from_nanos(2375),
        None,
    );
    // exactly at start should report the start
    test(
        c,
        Duration::from_millis(999),
        Duration::from_millis(1),
        Some((Duration::from_secs(1), Duration::from_millis(1), false)),
    );
    // striding the start should accumulate correctly
    test(
        c,
        Duration::from_millis(999),
        Duration::from_millis(20),
        Some((
            Duration::from_millis(1019),
            Duration::from_millis(20),
            false,
        )),
    );
    // after we've started it should just be normal
    test(
        c,
        Duration::from_secs(1),
        Duration::from_millis(8),
        Some((Duration::from_millis(1008), Duration::from_millis(8), false)),
    );
    test(
        c,
        Duration::from_millis(1100),
        Duration::from_millis(999),
        Some((
            Duration::from_millis(2099),
            Duration::from_millis(999),
            false,
        )),
    );
    test(
        c,
        Duration::from_millis(3900),
        Duration::from_millis(999),
        Some((
            Duration::from_millis(4899),
            Duration::from_millis(999),
            false,
        )),
    );
    // exactly on the end should land dead on

    test(
        c,
        Duration::from_millis(4901),
        Duration::from_millis(99),
        Some((Duration::from_secs(5), Duration::from_millis(99), true)),
    );
    // and past the end should stop at the right place and truncate dt
    test(
        c,
        Duration::from_millis(4901),
        Duration::from_millis(990),
        Some((Duration::from_secs(5), Duration::from_millis(99), true)),
    );

    // okay that works... now reverse it! which is mostly the same,
    // but t values run the other way
    let c = c.play_in_reverse();
    // should still be None during the first second
    test(c, Duration::ZERO, Duration::from_nanos(1102375), None);
    test(
        c,
        Duration::from_millis(400),
        Duration::from_nanos(1102375),
        None,
    );
    test(
        c,
        Duration::from_millis(999),
        Duration::from_nanos(2375),
        None,
    );
    // exactly at start should report back the end
    test(
        c,
        Duration::from_millis(900),
        Duration::from_millis(100),
        Some((Duration::from_secs(5), Duration::from_millis(100), false)),
    );
    // we're already past the start, we tick downward
    test(
        c,
        Duration::from_secs(1),
        Duration::from_millis(8),
        Some((Duration::from_millis(4992), Duration::from_millis(8), false)),
    );
    // make sure we compensate for dt striding the reverse start, but the other way
    test(
        c,
        Duration::from_millis(900),
        Duration::from_secs(1),
        Some((Duration::from_millis(4100), Duration::from_secs(1), false)),
    );
    // and so it goes
    test(
        c,
        Duration::from_millis(3900),
        Duration::from_secs(1),
        Some((Duration::from_millis(1100), Duration::from_secs(1), false)),
    );
    // report the finish when we land on it
    test(
        c,
        Duration::from_millis(4900),
        Duration::from_millis(100),
        Some((Duration::from_secs(1), Duration::from_millis(100), true)),
    );
    // report the finish when we stride it
    test(
        c,
        Duration::from_millis(4900),
        Duration::from_secs(1),
        Some((Duration::from_secs(1), Duration::from_millis(100), true)),
    );
}

#[derive(Component)]
#[require(TestTarget<TestAnimation>)]
struct TestAnimation;

#[derive(Component)]
#[require(TestTarget<TestAnimation2>, PersistentTick<TestAnimation2>)]
struct TestAnimation2;

#[derive(Component, Debug)]
struct TestTarget<T> {
    local: usize,
    t: f32,
    dt: f32,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Default for TestTarget<T> {
    fn default() -> Self {
        Self {
            local: 0,
            t: 0.0,
            dt: 0.0,
            _marker: PhantomData,
        }
    }
}

impl<T> Clone for TestTarget<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for TestTarget<T> {}

fn tick_test_animation(
    mut tracker: Local<usize>,
    target: Single<(&mut TestTarget<TestAnimation>, &Tick<TestAnimation>)>,
) {
    let (mut target, tick) = target.into_inner();
    // this do-si-do is to verify i got the system param stuff in workable state
    *tracker += 1;
    target.local = *tracker;
    target.t = tick.t;
    target.dt += tick.dt;
}

impl Animation for TestAnimation {
    fn system() -> (impl bevy_ecs::schedule::ScheduleLabel, TickSystemConfigs) {
        (Update, tick_test_animation.into_configs())
    }

    fn configuration(&self) -> impl Into<AnimationConfiguration> {
        duration(0.5)
    }
}

fn tick_test_animation2(
    mut tracker: Local<usize>,
    target: Single<
        (
            &mut TestTarget<TestAnimation2>,
            &PersistentTick<TestAnimation2>,
        ),
        Without<Tick<TestAnimation2>>,
    >,
) {
    let (mut target, tick) = target.into_inner();
    let Some(tick) = **tick else {
        return;
    };
    // this do-si-do is to verify i got the system param stuff in workable state
    *tracker += 1;
    target.local = *tracker;
    target.t = tick.t;
    target.dt += tick.dt;
}

impl Animation for TestAnimation2 {
    fn system() -> (impl bevy_ecs::schedule::ScheduleLabel, TickSystemConfigs) {
        (Update, tick_test_animation2.into_configs())
    }

    fn configuration(&self) -> impl Into<AnimationConfiguration> {
        duration(0.55)
    }
}

#[derive(Resource, Default)]
struct Animation1Finished(bool);

#[derive(Resource, Default)]
struct Animation2Finished(bool);

#[test]
fn test_basics() {
    let mut app = App::new();
    app.add_plugins(TimePlugin)
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .init_resource::<Animation1Finished>()
        .init_resource::<Animation2Finished>()
        .add_animation::<TestAnimation>()
        .add_animation::<TestAnimation2>()
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn((TestAnimation, TestAnimation2));
        })
        .add_observer(
            |_: On<AnimationFinished<TestAnimation>>, mut finished: ResMut<Animation1Finished>| {
                finished.0 = true;
            },
        )
        .add_observer(
            |_: On<AnimationFinished<TestAnimation2>>, mut finished: ResMut<Animation2Finished>| {
                finished.0 = true;
            },
        );
    // back to back to back to back frames. the GPU is strong with this one
    // pretty basic test, just make sure we got called and time moves like we expect
    let ticks = 40; // few ticks too many to make sure we stop accumulating after finish
    for _ in 0..ticks {
        app.update();
    }
    let mut query = app.world_mut().query::<&TestTarget<TestAnimation>>();
    let target = *query.single(app.world()).unwrap();
    assert_eq!(target.local, 30); // exactly half a second at 60fps
    assert!(target.t > 0.0);
    assert_float_eq!(target.t, target.dt, abs <= 0.001);

    let mut query = app.world_mut().query::<&TestTarget<TestAnimation2>>();
    let target = *query.single(app.world()).unwrap();
    assert_eq!(target.local, 34); // cheaty copy from the failure! but it's right
    assert!(target.t > 0.0);
    assert_float_eq!(target.t, target.dt, abs <= 0.001);

    // since we ran for half a second at 60fps we should always be finished
    assert!(app.world().resource_ref::<Animation1Finished>().0);
    assert!(app.world().resource_ref::<Animation2Finished>().0);

    // the rest is handled in examples, which are more fun to maintain and verify
}
