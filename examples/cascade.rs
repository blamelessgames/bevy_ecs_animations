//! Example of UI menu items timing their animation based on the y-height of
//! the screen, creating a cascading effect
use bevy::prelude::*;
use bevy_ecs_animations::*;

/// Duration of the local animation for a UI Element
const DURATION_SECONDS: f32 = 0.5;

/// The longest that an element will delay before its animation is triggered.
const MAX_DELAY_SECONDS: f32 = 1.5;

#[derive(Component, Default, Clone, Copy)]
pub struct CascadeIn;

impl Animation for CascadeIn {
    fn configuration(&self) -> impl Into<AnimationConfiguration> {
        // Because the animation is delayed the further down the screen the
        // UI element is, and all the animations have to be the same length,
        // make the animation lengths be long enough to fit the longest
        // possible animation.
        AnimationConfiguration::from(DURATION_SECONDS + MAX_DELAY_SECONDS)
            // Repeat the animation in order to demonstrate its effect.
            // Remove this for actual use.
            .repeat(5000)
    }

    fn system() -> (impl bevy_ecs::schedule::ScheduleLabel, TickSystemConfigs) {
        (Update, cascade_in.into_configs())
    }
}

fn cascade_in(
    window: Single<&Window>,
    mut anim: Query<(
        &Tick<CascadeIn>,
        &mut TextColor,
        &mut UiTransform,
        &UiGlobalTransform,
    )>,
) {
    for (tick, mut color, mut transform, global_transform) in &mut anim {
        // Get the normalized y as a way to trigger animations based on this
        // element's position on the screen, with the value between 0.0 and 1.0.
        let y_norm = global_transform.translation.y / window.resolution.height();

        // Delay based on the normalized y position
        let delay_seconds = y_norm * MAX_DELAY_SECONDS;

        // Time for this entity, with empty space before and after the animation
        let t_local = (tick.t - delay_seconds).clamp(0.0, DURATION_SECONDS);
        let t_local_normalized = t_local / DURATION_SECONDS;

        let ease_in = EaseFunction::CubicIn.sample_unchecked(t_local_normalized);
        let ease_out = EaseFunction::CubicOut.sample_unchecked(t_local_normalized);
        let inverted_ease_out = 1.0 - ease_out;

        color.set_alpha(ease_in);
        transform.translation.y = percent(50.0 * inverted_ease_out);
        transform.scale.x = 1.0 + (0.5 * inverted_ease_out);
        transform.scale.y = ease_out;
    }
}

// Main and scene setup
fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_animation::<CascadeIn>()
        .add_systems(Startup, (camera.spawn(), menu.spawn()))
        .run()
}

fn camera() -> impl Scene {
    bsn! {
        Camera2d
        Camera {
            clear_color: Color::BLACK,
        }
    }
}

pub fn menu() -> impl Scene {
    bsn! {
        #Menu
        Node {
            width: percent(100),
            height: percent(100),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Start
            flex_direction: FlexDirection::Column,
            padding: percent(10),
        }
        Children [
            menu_button("Option 1"),
            menu_button("Option 2"),
            menu_button("Option 3"),
            menu_button("Option 4"),
            menu_button("Option 5"),
            menu_button("Option 6"),
            menu_button("Option 7"),
            menu_button("Option 8"),
            menu_button("Option 9"),
            menu_button("Option 10"),
        ]
    }
}

pub fn menu_button(name: &'static str) -> impl Scene {
    bsn! {
        #MenuButton
        Button
        Node
        Children [
            (
                CascadeIn // the Animation component
                Text({name})
                TextFont {
                    font_size: FontSize::Vh(7.0),
                }
            ),
        ]
    }
}
