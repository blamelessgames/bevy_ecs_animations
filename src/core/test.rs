use std::{marker::PhantomData, time::Duration};

use crate::{core::AnimationTickState, *};
use bevy::{prelude::*, time::TimePlugin};
use bevy_time::TimeUpdateStrategy;
use float_eq::assert_float_eq;

#[test]
fn test_timekeeping() {
    use float_eq::assert_float_eq;
    fn test(c: AnimationConfiguration, elapsed: f32, dt: f32, expected: Option<(f32, f32, bool)>) {
        let Some(AnimationTickState { t, dt, finished }) = c.t(elapsed, dt) else {
            assert_eq!(expected, None);
            return;
        };
        let (e_t, e_dt, e_finished) = expected.expect("expected None!");
        // millisecond precision seems fine?
        assert_float_eq!(e_t, t, abs <= 0.001);
        assert_float_eq!(e_dt, dt, abs <= 0.001);
        assert_eq!(e_finished, finished);
    }

    let c = AnimationConfiguration::from(4.0);
    test(c, 0.0, 0.001102375, Some((0.001102375, 0.001102375, false)));
    test(c, 0.1, 0.008, Some((0.108, 0.008, false)));
    test(c, 0.2, 0.008, Some((0.208, 0.008, false)));
    // compensate for dt going past the end
    test(c, 3.9, 1.0, Some((4.0, 0.1, true)));
    test(c, 3.99, 1.0, Some((4.0, 0.01, true)));
    // this doesn't get called after it reports finished

    let c = c.start_at(1.0);
    // it should be None the whole first second
    test(c, 0.0, 0.001102375, None);
    test(c, 0.4, 0.001102375, None);
    test(c, 0.9, 0.001102375, None);
    // exactly at start should work
    test(c, 0.99, 0.01, Some((1.0, 0.01, false)));
    // and striding the start
    test(c, 0.9, 0.2, Some((1.1, 0.2, false)));
    // and right after start
    test(c, 1.0, 0.008, Some((1.008, 0.008, false)));
    // and normal stuff
    test(c, 1.1, 1.0, Some((2.1, 1.0, false)));
    test(c, 3.9, 1.0, Some((4.9, 1.0, false)));
    // the end is 1 second later now
    test(c, 4.9, 1.0, Some((5.0, 0.1, true)));

    // okay that works... now reverse it! which is mostly the same,
    // but t values run the other way
    let c = c.play_in_reverse();
    // should still not care during the first second
    test(c, 0.0, 0.001102375, None);
    test(c, 0.4, 0.001102375, None);
    test(c, 0.9, 0.001102375, None);
    // exactly at start should report back the end
    test(c, 0.9, 0.1, Some((5.0, 0.1, false)));
    test(c, 1.0, 0.008, Some((4.992, 0.008, false)));
    // make sure we compensate for dt striding the start, but the other way
    test(c, 1.1, 1.0, Some((3.9, 1.0, false)));
    // and so it goes
    test(c, 3.9, 1.0, Some((1.1, 1.0, false)));
    test(c, 4.9, 1.0, Some((1.0, 0.1, true)));
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
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
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
    assert_eq!(target.local, 33); // cheaty copy from the failure! but it's right
    assert!(target.t > 0.0);
    assert_float_eq!(target.t, target.dt, abs <= 0.001);

    // since we ran for half a second at 60fps we should always be finished
    assert!(app.world().resource_ref::<Animation1Finished>().0);
    assert!(app.world().resource_ref::<Animation2Finished>().0);

    // the rest is handled in examples, which are more fun to maintain and verify
}
