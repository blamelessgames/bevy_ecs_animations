use std::f32::consts::*;

use bevy::{color::palettes::css::*, prelude::*};
use bevy_ecs_animations::{
    EntityAnimation, EntityAnimationController, EntityAnimationPlugin, map, scaled_domain,
    scaled_output,
};

fn main() -> AppExit {
    App::new()
        .add_plugins((
            DefaultPlugins,
            EntityAnimationPlugin::<Wobble>::default(),
            EntityAnimationPlugin::<Spin>::default(),
        ))
        .add_systems(Startup, startup)
        .add_systems(PreUpdate, input)
        .run()
}

fn input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut spin_controller: EntityAnimationController<Spin>,
    mut wobble_controller: EntityAnimationController<Wobble>,
    mut config_store: ResMut<GizmoConfigStore>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        spin_controller.flip_pause_all();
        wobble_controller.flip_pause_all();
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

    commands.spawn((
        Mesh3d(meshes.add(Capsule3d::new(1.0, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: WHITE_SMOKE.into(),
            metallic: 0.9,
            perceptual_roughness: 0.2,

            ..default()
        })),
        Wobble::default(),
    ));
}

#[derive(Component)]
struct Spin;

impl EntityAnimation for Spin {
    // use whatever components you like
    type QueryData = &'static mut Transform;
    // you'll usually want this
    type QueryFilter = With<Self>;

    fn duration(&self) -> f32 {
        12.0
    }

    // repeat of 0 means "run once"
    // so 9 means 10, in case you wanted
    // more fence post fun
    // i might clean this up lol
    fn repeat(&self) -> u32 {
        9
    }

    fn tick(
        &mut self,
        _: f32,
        dt: f32,
        _: Commands,
        entity: Entity,
        mut components: Query<Self::QueryData, Self::QueryFilter>,
    ) {
        let Ok(mut transform) = components.get_mut(entity) else {
            return;
        };
        // you don't need to use a curve
        // obviously there are about 50 ways to achieve this same animation
        let new = (Rot2::radians(dt * 0.5) * transform.translation.xy()).normalize();
        *transform = Transform::from_translation(new.extend(3.0)).looking_at(Vec3::ZERO, Vec3::Y);
    }
}

#[derive(Component, Deref)]
struct Wobble(Box<dyn Curve<Dir3> + Send + Sync + 'static>);

impl Default for Wobble {
    fn default() -> Self {
        Wobble(Box::new(map(
            scaled_domain(
                0.0,
                120.0,
                scaled_output(TAU * -20.0, TAU * 20.0, EaseFunction::CircularInOut),
            ),
            |angle| Dir3::new_unchecked(Dir3::X.rotate_z(angle).normalize()),
        )))
    }
}

impl EntityAnimation for Wobble {
    // use whatever components you like
    type QueryData = &'static mut Transform;
    // you'll usually want this
    type QueryFilter = With<Self>;

    fn duration(&self) -> f32 {
        120.0
    }

    fn tick(
        &mut self,
        t: f32,
        dt: f32,
        _: Commands,
        entity: Entity,
        mut components: Query<Self::QueryData, Self::QueryFilter>,
    ) {
        let Ok(mut transform) = components.get_mut(entity) else {
            return;
        };
        // do whatever you want, get ticked on schedule til duration is up
        transform.rotate_axis(self.sample_unchecked(t), dt * 2.0);
    }
}
