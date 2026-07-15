//! basic example
use std::ops::Deref;

use bevy::prelude::*;
use bevy_ecs_animations::*;

// 1. Define a component for your animation. It's a regular Component,
//    so you can do regular Component things like give it state that you
//    use in systems
#[derive(Component)]
#[allow(unused)]
enum Fade {
    In(EaseFunction, f32),
    Out(EaseFunction, f32),
}

impl Deref for Fade {
    type Target = EaseFunction;
    fn deref(&self) -> &Self::Target {
        match self {
            Fade::In(ease, _) => ease,
            Fade::Out(ease, _) => ease,
        }
    }
}

// 2. Define a system that will tick the animation. Your system should
//    query &Tick<YourAnim> to receive timing data. Any kind of query works
fn tick_fade(fade: Single<(&mut TextColor, &Tick<Fade>, &Fade)>) {
    // The `Tick` component is a per-animation-type component that is present
    // on the entity while the animation is active, and it exposes the
    // state of the animation while it is active
    let (mut text_color, tick, fade) = fade.into_inner();
    // Bevy ease functions expect unit input, so here we use `normalized_t`
    // the `Tick` component exposes raw `t` and `dt` as well
    let alpha = fade.sample_unchecked(tick.normalized_t);
    text_color.set_alpha(alpha);
}

// 3. implement `Animation`
impl Animation for Fade {
    // 3a. provide the schedule and system that will be receive ticks.
    fn system() -> (impl bevy_ecs::schedule::ScheduleLabel, TickSystemConfigs) {
        (
            // this can be any schedule you like, of course
            Update,
            // dealing with systems generically is a little tricky but if you invoke
            // `into_configs()` it works
            tick_fade.into_configs(),
        )
    }

    // animations require a configuration, minimally a duration
    // since f32 implements Into<AnimationConfiguration> as the duration,
    // you can just return that if you're happy with the other defaults, but
    // we're having a little fun with it
    fn configuration(&self) -> impl Into<AnimationConfiguration> {
        match *self {
            Fade::In(_, duration) => AnimationConfiguration::duration(duration).play_forward(),
            Fade::Out(_, duration) => AnimationConfiguration::duration(duration).play_in_reverse(),
        }
    }
}

// 4. Set up your app, registering your animation component so the tick system runs
fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        // this step is important! If you forget it, the animation won't do anything and there
        // will be no messages explaining why.
        .add_animation::<Fade>()
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
        Fade::In(EaseFunction::CubicIn, 3.5),
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
