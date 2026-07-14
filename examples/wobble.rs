//! lots of animations doing things
use std::f32::consts::*;

use bevy::{color::palettes::css::*, prelude::*};
use bevy_ecs::schedule::ScheduleLabel;
use bevy_ecs_animations::{
    AnimationConfiguration, ECSAnimation, ECSAnimationCommands, ECSAnimationConfigs,
    ECSAnimationFinished, ECSAnimationsApp, Tick,
    combinators::{BoxedCurve, map, scaled_output},
};

// 15 minutes oughta be enough for anybody
const TOTAL_TIME: f32 = 900.0;

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        .register_ecs_animation::<Wobble>()
        .register_ecs_animation::<Spin>()
        .register_ecs_animation::<Fade>()
        .add_systems(Startup, startup)
        .add_systems(PreUpdate, input)
        .run()
}

fn input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut config_store: ResMut<GizmoConfigStore>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        commands.flip_pause_all::<Wobble>().flip_pause_all::<Spin>();
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
            Wobble::default(),
        ))
        .observe(
            // you can observe entities for completion or the start of new repetitions
            |finished: On<ECSAnimationFinished<Fade>>,
             fade: Single<&Fade>,
             mut commands: Commands| {
                if **fade == Fade::Out {
                    commands.entity(finished.event_target()).insert(Fade::In);
                }
            },
        );
}

#[derive(Component, PartialEq, Eq)]
enum Fade {
    Out,
    In,
}

fn tick_fade(
    mut materials: ResMut<Assets<StandardMaterial>>,
    faded: Single<(&MeshMaterial3d<StandardMaterial>, &Tick<Fade>)>,
) {
    let (mesh_material, tick) = faded.into_inner();

    let Some(mut material) = materials.get_mut(mesh_material.0.id()) else {
        return;
    };
    // because we're scaling a simple unit function we use `normalized_t`
    material
        .base_color
        .set_alpha(EaseFunction::QuadraticIn.sample_clamped(tick.normalized_t));
}

impl ECSAnimation for Fade {
    fn system() -> (impl ScheduleLabel, ECSAnimationConfigs) {
        (Update, tick_fade.into_configs())
    }

    fn configuration(&self) -> impl Into<AnimationConfiguration> {
        match self {
            Fade::Out => AnimationConfiguration::from(1.4)
                .start_at(2.1)
                .play_in_reverse()
                .remove_nothing(),
            Fade::In => AnimationConfiguration::from(1.2).start_at(0.5),
        }
    }
}

#[derive(Component)]
struct Spin;

fn tick_spin(spinners: Query<(&mut Transform, &Tick<Spin>)>) {
    for (mut transform, tick) in spinners {
        // you don't need to use a curve
        // obviously there are about 50 million ways to achieve this same animation
        let new = (Rot2::radians(tick.dt * 0.5) * transform.translation.xy()).normalize();
        *transform = Transform::from_translation(new.extend(3.0)).looking_at(Vec3::ZERO, Vec3::Y);
    }
}

impl ECSAnimation for Spin {
    fn system() -> (impl ScheduleLabel, ECSAnimationConfigs) {
        (Update, tick_spin.into_configs())
    }

    fn configuration(&self) -> impl Into<AnimationConfiguration> {
        AnimationConfiguration::from(12.0).repeat((TOTAL_TIME / 12.0) as u32)
    }
}

#[derive(Component, Deref)]
struct Wobble(BoxedCurve<Dir3>);

fn tick_wobble(wobble: Single<(&mut Transform, &Wobble, &Tick<Wobble>)>) {
    let (mut transform, wobble, tick) = wobble.into_inner();
    transform.rotate_axis(wobble.sample_unchecked(tick.normalized_t), tick.dt * 2.0);
}

impl Default for Wobble {
    fn default() -> Self {
        Wobble(Box::new(map(
            scaled_output(TAU * -10.0, TAU * 10.0, EaseFunction::CircularInOut),
            |angle| Dir3::new_unchecked(Dir3::X.rotate_z(angle).normalize()),
        )))
    }
}
impl ECSAnimation for Wobble {
    fn system() -> (impl ScheduleLabel, ECSAnimationConfigs) {
        (Update, tick_wobble.into_configs())
    }

    fn configuration(&self) -> impl Into<AnimationConfiguration> {
        TOTAL_TIME
    }
}
