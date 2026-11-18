//! Demonstrates using non-default `Time` contexts and `Duration` properties
use std::time::Duration;

use bevy::{color::palettes::css::*, prelude::*};

use bevy_ecs_animations::*;

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_animation::<XScale>()
        .add_animation_in_time_context::<YScale, Real>()
        .add_systems(Startup, startup)
        .add_systems(PreUpdate, keys)
        .run()
}

fn startup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        AmbientLight {
            color: WHITE.into(),
            brightness: 10000.0,
            ..default()
        },
        Transform::from_translation(vec3(0.0, 0.0, 10.0)).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(3.0, 3.0, 3.0))),
        MeshMaterial3d(materials.add(Color::from(GREEN))),
        XScale,
        YScale,
    ));
}

fn keys(keyboard: Res<ButtonInput<KeyCode>>, mut time: ResMut<Time<Virtual>>) {
    if keyboard.just_pressed(KeyCode::Space) {
        if time.is_paused() {
            time.unpause();
        } else {
            time.pause();
        }
    }

    const MAX: f64 = 3.0;
    const MIN: f64 = 0.1;

    let mut speed = time.relative_speed_f64();

    if keyboard.pressed(KeyCode::ArrowUp) {
        speed += 0.1
    }

    if keyboard.pressed(KeyCode::ArrowDown) {
        speed -= 0.1;
    }

    time.set_relative_speed_f64(speed.clamp(MIN, MAX));
}

#[derive(Component)]
struct XScale;

#[animation(
    config = duration_time(Duration::from_secs(u64::MAX))
)]
fn tick_x_scale(target: Single<(&mut Transform, &Tick<XScale>)>) {
    let (mut transform, tick) = target.into_inner();
    transform.scale.x = (tick.time.as_secs_f64().cos() * 0.5 + 1.0) as f32;
}

#[derive(Component)]
struct YScale;

#[animation(
    config = duration_time(Duration::from_secs(u64::MAX)).delay_by_time(Duration::from_secs(2))
)]
fn tick_y_scale(target: Single<(&mut Transform, &Tick<YScale>)>) {
    let (mut transform, tick) = target.into_inner();
    transform.scale.y = (tick.time.as_secs_f64().sin() * 0.5 + 1.0) as f32;
}
