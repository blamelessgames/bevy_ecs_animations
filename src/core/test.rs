use crate::*;
use bevy::{prelude::*, time::TimePlugin};
use float_eq::assert_float_eq;

#[test]
fn test_timekeeping() {
    use float_eq::assert_float_eq;
    fn test(c: AnimationConfiguration, elapsed: f32, dt: f32, expected: Option<(f32, f32, bool)>) {
        let Some((elapsed, dt, done)) = c.t(elapsed, dt) else {
            assert_eq!(expected, None);
            return;
        };
        let (e_elapsed, e_dt, e_done) = expected.expect("expected None!");
        // millisecond precision seems fine?
        assert_float_eq!(e_elapsed, elapsed, abs <= 0.001);
        assert_float_eq!(e_dt, dt, abs <= 0.001);
        assert_eq!(e_done, done);
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
#[require(TestTarget)]
struct TestAnimation {
    duration: f32,
}

#[derive(Component, Default, Debug)]
struct TestTarget {
    local: usize,
    t: f32,
    dt: f32,
}

fn tick_test_animation(
    mut tracker: Local<usize>,
    target: Single<(&mut TestTarget, &Tick<TestAnimation>)>,
) {
    let (mut target, tick) = target.into_inner();
    // this do-si-do is to verify i got the system param stuff in workable state
    *tracker += 1;
    target.local = *tracker;
    target.t = tick.t;
    target.dt += tick.dt;
}

impl ECSAnimation for TestAnimation {
    fn system() -> (impl bevy_ecs::schedule::ScheduleLabel, ECSAnimationConfigs) {
        (Update, tick_test_animation.into_configs())
    }

    fn configuration(&self) -> impl Into<AnimationConfiguration> {
        self.duration
    }
}

#[test]
fn test_basics() {
    let mut app = App::new();
    app.add_plugins(TimePlugin)
        .register_ecs_animation::<TestAnimation>()
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(TestAnimation { duration: 1.0 });
        });
    // back to back to back to back frames. the GPU is strong with this one
    app.update();
    app.update();
    app.update();
    app.update();
    let mut query = app.world_mut().query::<&TestTarget>();
    let target = query.single(app.world()).unwrap();
    // pretty basic test, just make sure we got called and time moves like we expect
    // (exact times depend on the timing of calling update so we aren't going to check that, but
    // more than zero, and dt accumulation ~ t checks things work as expected)
    assert_eq!(target.local, 4);
    assert!(target.t > 0.0);
    assert_float_eq!(target.t, target.dt, abs <= 0.001);

    // the rest is handled in examples, which are more fun to maintain and verify
}
