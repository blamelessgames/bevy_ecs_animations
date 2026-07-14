#![allow(refining_impl_trait)]
use bevy_ecs::system::{StaticSystemParam, lifetimeless::Write};
use float_eq::assert_float_eq;
use std::ops::DerefMut;

use super::*;
use bevy::prelude::*;
use bevy_time::TimePlugin;

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

impl EntityAnimation for TestAnimation {
    type Param = (
        SSingle<Write<TestTarget>, With<TestAnimation>>,
        SLocal<usize>,
    );

    fn configuration(&self) -> f32 {
        self.duration
    }

    fn tick(
        &mut self,
        _entity: Entity,
        t: f32,
        dt: f32,
        param: &mut StaticSystemParam<Self::Param>,
    ) {
        let (target, tracker) = param.deref_mut();
        // this do-si-do is to verify i got the system param stuff in workable state
        **tracker += 1;
        target.local = **tracker;
        target.t = t;
        target.dt += dt;
    }
}

#[test]
fn test_basics() {
    let mut app = App::new();
    app.add_plugins((
        TimePlugin,
        EntityAnimationPlugin::<TestAnimation>::default(),
    ))
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
