use std::f32::consts::*;

use bevy::{color::palettes::css::*, prelude::*};
use bevy_ecs_animations::{EntityAnimation, EntityAnimationPlugin, scaled_domain, scaled_output};

fn main() -> AppExit {
    App::new()
        .add_plugins((DefaultPlugins, EntityAnimationPlugin::<Spinner>::default()))
        .add_systems(Startup, startup)
        .run()
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
        PointLight {
            color: DEEP_PINK.into(),
            intensity: 100_000.0,
            ..default()
        },
        Transform::from_translation(vec3(2.0, 2.0, 2.0)).looking_at(Vec3::ZERO, Dir3::Y),
    ));

    commands.spawn((
        PointLight {
            color: REBECCA_PURPLE.into(),
            intensity: 100_000.0,
            ..default()
        },
        Transform::from_translation(vec3(-2.0, -2.0, -2.0)).looking_at(Vec3::ZERO, Dir3::Y),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: GOLDENROD.into(),
            metallic: 1.0,
            clearcoat: 1.0,
            ..default()
        })),
        Spinner,
    ));
}

#[derive(Component)]
struct Spinner;

impl Spinner {
    // if you declare curves in const functions that return
    // `impl Curve<T>` you can ensure cheap inline construction
    // in the tick function, saving the trouble of trying to write
    // the insane types combinators will produce

    // of course the bevy curve combinators also work just fine
    // if you use other approaches.
    const fn axis_curve() -> impl Curve<f32> {
        // adapt a curve using combinators.
        // the domain of the curve is the timeline
        // the output of the curve is whatever you want it to be really
        scaled_domain(
            0.0,
            120.0,
            scaled_output(TAU * -20.0, TAU * 20.0, EaseFunction::CircularInOut),
        )
    }
}

impl EntityAnimation for Spinner {
    type QueryData = &'static mut Transform;

    type QueryFilter = ();

    fn duration(&self) -> f32 {
        120.0
    }

    fn tick(&mut self, t: f32, dt: f32, entity: Entity, mut components: Query<&mut Transform>) {
        let Ok(mut transform) = components.get_mut(entity) else {
            return;
        };
        // do whatever you want, get ticked on schedule til duration is up
        let axis = Dir3::Y
            .rotate_z(Spinner::axis_curve().sample_unchecked(t))
            .normalize();
        transform.rotate_axis(Dir3::new_unchecked(axis), dt * 2.0);
    }
}
