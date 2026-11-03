//! basic example
use std::ops::Deref;

use bevy::prelude::*;
use bevy_ecs_animations::*;

// 1. Define a component for your animation. It's a regular Component,
//    so you can do regular Component things like give it state that you
//    use in systems
#[derive(Component)]
#[allow(unused)]
enum FadeText {
    In(EaseFunction, f32),
    Out(EaseFunction, f32),
}

impl Deref for FadeText {
    type Target = EaseFunction;
    fn deref(&self) -> &Self::Target {
        match self {
            FadeText::In(ease, _) => ease,
            FadeText::Out(ease, _) => ease,
        }
    }
}

// 2. Define a system that will tick the animation. Your system should
//    query &Tick<...> to receive timing data. Any kind of query works.
//    Use the `animation` macro to tie the type to the system
//    by implementing the `Animation` trait
#[animation(config = match *self {
    FadeText::In(_, d) => duration(d).play_forward(),
    FadeText::Out(_, d) => duration(d).play_in_reverse()
})]
fn tick_fade_text(fade: Single<(&mut TextColor, &Tick<FadeText>, &FadeText)>) {
    // The `Tick` component is a per-animation-type component that is present
    // on the entity while the animation is active, and it exposes the
    // state of the animation while it is active
    let (mut text_color, tick, fade) = fade.into_inner();
    // Bevy ease functions expect unit input, so here we use `normalized_t` and `sample_unchecked`
    // the `Tick` component exposes raw `t` and `dt` as well
    let alpha = fade.sample_unchecked(tick.normalized_t);
    text_color.set_alpha(alpha);
}

// 3. Set up your app, registering your animation component so the tick system runs
fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        // this step is important! If you forget it, the animation won't do anything and there
        // will be no messages explaining why.
        .add_animation::<FadeText>()
        .add_systems(Startup, startup)
        .run()
}

// 4. Spawn an animation on an entity and it'll tick
fn startup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: Color::BLACK.into(),
            ..default()
        },
    ));
    commands.spawn((
        // Inserting the component on an entity will start the animation
        FadeText::In(EaseFunction::CubicIn, 3.5),
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
