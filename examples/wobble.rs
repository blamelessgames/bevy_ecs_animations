//! a bunch of animations doing things without much ceremony
use std::f32::consts::*;

use bevy::{color::palettes::css::*, prelude::*};
use bevy_ecs_animations::*;

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_animation::<Wobble>()
        .add_animation::<Spin>()
        .add_animation::<Fade>()
        .add_systems(Startup, startup)
        .add_systems(Update, input)
        .run()
}

#[derive(Component)]
#[require(PersistentTick<Spin>)]
struct Spin;

#[animation]
fn tick_spin(spinners: Query<(&mut Transform, &PersistentTick<Spin>)>) {
    for (mut transform, tick) in spinners {
        let Some(tick) = **tick else {
            continue;
        };
        let new = (Rot2::radians(tick.dt * 0.5) * transform.translation.xy()).normalize();
        *transform = Transform::from_translation(new.extend(3.0)).looking_at(Vec3::ZERO, Vec3::Y);
    }
}

#[derive(Component)]
#[require(PersistentTick<Wobble>)]
struct Wobble;

#[animation]
fn tick_wobble(wobbler: Single<(&mut Transform, &PersistentTick<Wobble>)>) {
    let (mut transform, tick) = wobbler.into_inner();
    let Some(tick) = **tick else {
        return;
    };
    // the default animation configuration is `forever` which is really 3600.0 seconds,
    // (repeated for a really long time), so treating t as degrees works out
    let radians = tick.t.to_radians();
    let axis = Dir3::new_unchecked(
        Dir3::X
            .rotate_z(
                // stack some cycles and wobble the night away
                (radians.sin()
                    + (radians * 10.832).sin() * 0.6
                    + (radians * 100.723).sin() * 0.3
                    + (radians * 999.284).sin() * 0.15)
                    * TAU,
            )
            .normalize(),
    );
    transform.rotate_axis(axis, tick.dt * 2.0);
}

#[derive(Component, PartialEq, Eq)]
enum Fade {
    Out,
    In,
}

#[animation(config = {
    match self {
        Fade::Out => duration(1.4)
            .start_at(2.1)
            .play_in_reverse()
            .remove_nothing(),
        Fade::In => duration(1.2).start_at(0.5),
    }
})]
fn tick_fade(
    mut materials: ResMut<Assets<StandardMaterial>>,
    faded: Single<(&MeshMaterial3d<StandardMaterial>, &Tick<Fade>)>,
) {
    let (mesh_material, tick) = faded.into_inner();

    let Some(mut material) = materials.get_mut(mesh_material.0.id()) else {
        return;
    };
    // because we're scaling a simple ease function we use `normalized_t`
    material
        .base_color
        .set_alpha(EaseFunction::QuadraticIn.sample_clamped(tick.normalized_t));
}

fn input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut config_store: ResMut<GizmoConfigStore>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        // if you're okay with deferred action you can control animations right from Commands
        commands
            .flip_pause_all::<Wobble>()
            .flip_pause_all::<Spin>()
            .flip_pause_all::<Fade>();
    }
    if keyboard.just_pressed(KeyCode::KeyG) {
        let (_, light_config) = config_store.config_mut::<LightGizmoConfigGroup>();
        // watch the lights move too!
        light_config.draw_all = !light_config.draw_all;
        light_config.color = LightGizmoColor::MatchLightColor;
    }
}

fn startup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(BLACK.into()),
            ..default()
        },
        Transform::from_translation(vec3(0.0, 0.0, 10.0)).looking_at(Vec3::ZERO, Dir3::Y),
    ));

    commands.spawn((
        DirectionalLight {
            color: DEEP_PINK.into(),
            ..default()
        },
        Transform::from_translation(vec3(2.0, 2.0, 0.0)).looking_at(Vec3::ZERO, Dir3::Y),
        // you can put the same animation on multiple entities, they all run independently
        Spin,
    ));

    commands.spawn((
        DirectionalLight {
            color: YELLOW_GREEN.into(),
            ..default()
        },
        Transform::from_translation(vec3(2.0, -2.0, 0.0)).looking_at(Vec3::ZERO, Dir3::Y),
        Spin,
    ));

    commands.spawn((
        DirectionalLight {
            color: MEDIUM_BLUE.into(),
            ..default()
        },
        Transform::from_translation(vec3(-2.0, 2.0, 0.0)).looking_at(Vec3::ZERO, Dir3::Y),
        Spin,
    ));

    commands.spawn((
        DirectionalLight {
            color: REBECCA_PURPLE.into(),
            ..default()
        },
        Transform::from_translation(vec3(-2.0, -2.0, 0.0)).looking_at(Vec3::ZERO, Dir3::Y),
        Spin,
    ));

    commands
        .spawn((
            Mesh3d(meshes.add(Capsule3d::new(1.0, 1.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: WHITE_SMOKE.with_alpha(1.0).into(),
                metallic: 0.9,
                perceptual_roughness: 0.2,
                alpha_mode: AlphaMode::Blend,
                ..default()
            })),
            // you can put multiple animations on an entity. There is no
            // automatic blending in this situation so don't target the
            // same properties with two animations at once unless you
            // like glitches and head-scratching bugs
            Fade::Out,
            Wobble,
        ))
        .observe(
            // you can observe entities for completion or the start of new repetitions
            |finished: On<AnimationFinished<Fade>>, fade: Single<&Fade>, mut commands: Commands| {
                if **fade == Fade::Out {
                    commands.entity(finished.event_target()).insert(Fade::In);
                }
            },
        );
}
