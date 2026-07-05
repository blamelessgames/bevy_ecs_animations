//! basic example
use bevy::{
    ecs::system::{StaticSystemParam, lifetimeless::*},
    prelude::*,
};
use bevy_ecs_animations::*;
use std::range::Range;

// 1. Define a component and implement EntityAnimation
#[derive(Component)]
struct FadeIn;

impl EntityAnimation for FadeIn {
    // Define the param your tick function receives,
    // using `bevy::ecs::system::lifetimeless` helpers
    type Param = SQuery<Write<TextColor>, With<Self>>;

    // Define the domain your animation runs. this is in seconds
    // and it starts ticking when the component is inserted
    fn domain(&self) -> Range<f32> {
        (0.0..4.0).into()
    }

    // Define the tick method, which will get invoked once
    // per frame until the domain is covered
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
        Camera2d::default(),
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
        Text::from("TEXT"),
        TextFont {
            font_size: FontSize::Vw(15.0),
            ..default()
        },
        TextLayout::justify(Justify::Center),
    ));
}
