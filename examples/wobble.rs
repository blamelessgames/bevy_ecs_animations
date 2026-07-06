//! lots of animations doing things
use std::{f32::consts::*, ops::DerefMut};

use bevy::{
    color::palettes::css::*,
    ecs::system::{
        StaticSystemParam,
        lifetimeless::{Read, SQuery, SResMut, Write},
    },
    prelude::*,
};
use bevy_ecs_animations::{
    AnimationCommands, AnimationConfiguration, EntityAnimation, EntityAnimationFinished,
    EntityAnimationPlugin,
    combinators::{BoxedCurve, map, scaled_output},
};

// 15 minutes oughta be enough for anybody
const TOTAL_TIME: f32 = 900.0;

fn main() -> AppExit {
    App::new()
        .add_plugins((
            DefaultPlugins,
            EntityAnimationPlugin::<Wobble>::default(),
            EntityAnimationPlugin::<Spin>::default(),
            EntityAnimationPlugin::<Fade>::default(),
        ))
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
            |finished: On<EntityAnimationFinished<Fade>>,
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

impl EntityAnimation for Fade {
    type Param = (
        SResMut<Assets<StandardMaterial>>,
        SQuery<Read<MeshMaterial3d<StandardMaterial>>>,
    );

    fn configuration(&self) -> impl Into<AnimationConfiguration> {
        match self {
            Fade::Out => AnimationConfiguration::from(1.4)
                .start_at(2.1)
                .play_in_reverse()
                .remove_nothing(),
            Fade::In => AnimationConfiguration::from(1.2).start_at(0.5),
        }
    }

    fn tick(&mut self, entity: Entity, t: f32, _: f32, param: &mut StaticSystemParam<Self::Param>) {
        // you can animate just about whatever your heart desires,
        let (materials, mesh_materials) = param.deref_mut();

        let Ok(MeshMaterial3d(handle)) = mesh_materials.get(entity) else {
            return;
        };
        let Some(mut material) = materials.get_mut(handle) else {
            return;
        };
        // because we're scaling a simple unit function
        // we have to normalize t
        let t = self.normalized_t(t);
        material
            .base_color
            .set_alpha(EaseFunction::QuadraticIn.sample_clamped(t));
    }
}

#[derive(Component)]
struct Spin;

impl EntityAnimation for Spin {
    type Param = SQuery<Write<Transform>, With<Self>>;

    fn configuration(&self) -> impl Into<AnimationConfiguration> {
        AnimationConfiguration::from(12.0).repeat((TOTAL_TIME / 12.0) as u32)
    }

    fn tick(
        &mut self,
        entity: Entity,
        _: f32,
        dt: f32,
        targets: &mut StaticSystemParam<Self::Param>,
    ) {
        let Ok(mut transform) = targets.get_mut(entity) else {
            return;
        };
        // you don't need to use a curve
        // obviously there are about 50 million ways to achieve this same animation
        let new = (Rot2::radians(dt * 0.5) * transform.translation.xy()).normalize();
        *transform = Transform::from_translation(new.extend(3.0)).looking_at(Vec3::ZERO, Vec3::Y);
    }
}

#[derive(Component, Deref)]
struct Wobble(BoxedCurve<Dir3>);

impl Default for Wobble {
    fn default() -> Self {
        Wobble(Box::new(map(
            scaled_output(TAU * -10.0, TAU * 10.0, EaseFunction::CircularInOut),
            |angle| Dir3::new_unchecked(Dir3::X.rotate_z(angle).normalize()),
        )))
    }
}
impl EntityAnimation for Wobble {
    type Param = SQuery<Write<Transform>, With<Self>>;

    fn configuration(&self) -> impl Into<AnimationConfiguration> {
        TOTAL_TIME
    }

    fn tick(
        &mut self,
        entity: Entity,
        t: f32,
        dt: f32,
        targets: &mut StaticSystemParam<Self::Param>,
    ) {
        let Ok(mut transform) = targets.get_mut(entity) else {
            return;
        };
        // do whatever you want, get ticked on schedule til duration is up
        transform.rotate_axis(self.sample_unchecked(self.normalized_t(t)), dt * 2.0);
    }
}
