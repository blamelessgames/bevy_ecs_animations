//! basic example
use bevy::{
    ecs::system::{StaticSystemParam, lifetimeless::*},
    prelude::*,
};
use bevy_ecs_animations::*;

// 1. Define a component and implement EntityAnimation
#[derive(Component)]
struct FadeIn;

impl EntityAnimation for FadeIn {
    // Define the param your tick function receives,
    // using `bevy::ecs::system::lifetimeless` helpers

    // this is effectively the same as the arguments to a system function,
    // but with 'static lifetimes so the generics work (Bevy uses correct
    // lifetimes at runtime). This gives your tick method full access to
    // the ECS in a way that lets it schedule tick systems to run in parallel
    // if possible
    type Param = SQuery<Write<TextColor>, With<Self>>;

    // animations require a configuration, minimally a duration
    // since f32 implements Into<AnimationConfiguration> as the duration,
    // you can just return that if you're happy with the other defaults
    fn configuration(&self) -> impl Into<AnimationConfiguration> {
        4.0
    }

    // This is the core. This function will get called every time the
    // `Update` schedule runs, with the entity the component is attached to,
    // the current spot in the timeline, and the system parameter. You can
    // animate just about anything using this approach, from transforms and
    // colors to which camera is active to entire lifecycles of entities that
    // run other animations. It's basically a specialized system.
    fn tick(
        &mut self,
        entity: Entity,
        t: f32,
        _dt: f32,
        param: &mut StaticSystemParam<Self::Param>,
    ) {
        let Ok(mut color) = param.get_mut(entity) else {
            return;
        };
        // Ease functions expect unit input, so normalize t first
        let t = self.normalized_t(t);
        let alpha = EaseFunction::CubicIn.sample_unchecked(t);
        color.set_alpha(alpha);
    }
}

// 2. Add a plugin for the animation component
fn main() -> AppExit {
    App::new()
        .add_plugins((DefaultPlugins, EntityAnimationPlugin::<FadeIn>::default()))
        .add_systems(Startup, startup)
        .run()
}

// 3. Spawn an animation on an entity in a system
fn startup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: Color::BLACK.into(),
            ..default()
        },
    ));
    commands.spawn((
        // Inserting the component on an entity starts the animation
        FadeIn,
        Node {
            width: percent(100.0),
            height: percent(100.0),
            padding: UiRect::top(percent(20.0)),
            ..default()
        },
        Text::from("Hello"),
        TextFont {
            font_size: FontSize::Vw(15.0),
            ..default()
        },
        TextLayout::justify(Justify::Center),
    ));
}
