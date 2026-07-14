//! basic example
use bevy::prelude::*;
use bevy_ecs_animations::*;

// 1. Define a component
#[derive(Component)]
struct FadeIn;

// 2. Define a system that will tick the animation. The `Tick` component is a per-animation-type
//    component that is present on the entity while the animation is active. This means if you only
//    have a single entity with a given component, using `Single` over the `Tick` will act as a run condition
fn tick_fade_in(fade_in: Single<(&mut TextColor, &Tick<FadeIn>)>) {
    let (mut text_color, tick) = fade_in.into_inner();
    // Bevy ease functions expect unit input, so use `normalized_t`
    let alpha = EaseFunction::CubicIn.sample_unchecked(tick.normalized_t);
    text_color.set_alpha(alpha);
}

// 3. implement `ECSAnimation`
impl ECSAnimation for FadeIn {
    // 3a. provide the schedule and system that will be receive ticks.
    fn system() -> (impl bevy_ecs::schedule::ScheduleLabel, ECSAnimationConfigs) {
        (
            // this can be any schedule you like, of course
            Update,
            // dealing with systems generically is a little tricky but if you invoke
            // `into_configs()` everything works
            tick_fade_in.into_configs(),
        )
    }

    // animations require a configuration, minimally a duration
    // since f32 implements Into<AnimationConfiguration> as the duration,
    // you can just return that if you're happy with the other defaults
    fn configuration(&self) -> impl Into<AnimationConfiguration> {
        4.0
    }
}

// 4. Set up your app, registering your animation components so they get ticks
fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        // this step is important!
        .register_ecs_animation::<FadeIn>()
        .add_systems(Startup, startup)
        .run()
}

// 4. Spawn an animation on an entity
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
