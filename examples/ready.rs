//! Demonstrates applying the same animation to several entities, using various animation
//! controller interfaces, and animating properties that bevy_animation makes somewhat annoying
use std::f32::consts::*;

use bevy::{ecs::schedule::ScheduleLabel, prelude::*};
use bevy_ecs_animations::{combinators::*, *};

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_animation::<ReadyLetter>()
        .add_animation::<PleaseSpawnReady>()
        .add_systems(Startup, startup)
        .add_observer(please_spawn_ready)
        .run()
}

#[derive(Component)]
struct Ready;

const READY: &str = "READY";

#[derive(Component)]
struct ReadyLetter(usize);

impl ReadyLetter {
    /// delay applied _before_ we start have a Tick available
    const fn delay(&self) -> f32 {
        // cheating here, a little bit of base delay to ensure the app gets started in non-optimized builds
        // before we try to animate things so it isn't hitching and out of order
        0.5 + self.0 as f32 * 0.15
    }

    /// length in seconds of the main animation without delay
    const fn main_phase_duration(&self) -> f32 {
        0.65
    }

    /// length in secdonds of fade animation phase
    const fn fade_phase_duration(&self) -> f32 {
        0.45
    }

    /// total duration
    const fn duration(&self) -> f32 {
        self.main_phase_duration() + self.fade_phase_duration()
    }

    const fn transform_curve(&self) -> impl Curve<(Val2, Vec2)> {
        scaled_domain(
            0.0,
            self.main_phase_duration(),
            zip(
                map(fn_curve(-PI, TAU - PI, |t| f32::cos(t) * 22.0), |y| {
                    Val2::px(0.0, y)
                }),
                map(
                    fn_curve(-PI, TAU - PI, |t| 1.5 + f32::sin(t * 1.24) * 0.5),
                    Vec2::splat,
                ),
            ),
        )
    }

    const fn text_color_curve(&self) -> impl Curve<Hsla> {
        seq(
            scaled_domain(
                0.0,
                self.main_phase_duration(),
                map(
                    zip(
                        scaled_output(0.0, 360.0, EaseFunction::SmootherStepIn),
                        EaseFunction::BounceOut,
                    ),
                    |(hue, alpha)| Hsla::new(hue, 0.5, 0.5, alpha),
                ),
            ),
            map(
                scaled_domain(
                    self.main_phase_duration() + ((READY.len() - self.0) as f32 * 0.08),
                    self.fade_phase_duration() + self.main_phase_duration(),
                    EaseFunction::CubicOut,
                ),
                |alpha| Hsla::new(360.0, 0.5, 0.5, 1.0 - alpha),
            ),
        )
    }
}

fn tick_ready_letter(
    mut ready_letters: Query<(
        &ReadyLetter,
        &mut UiTransform,
        &mut TextColor,
        &Tick<ReadyLetter>,
    )>,
    ready_letters_controller: AnimationController<ReadyLetter>,
    ready: Single<Entity, With<Ready>>,
    mut commands: Commands,
) {
    for (ready_letter, mut transform, mut text_color, tick) in ready_letters.iter_mut() {
        let (translation, scale) = ready_letter.transform_curve().sample_clamped(tick.t);
        transform.translation = translation;
        transform.scale = scale;
        text_color.0 = ready_letter
            .text_color_curve()
            .sample_clamped(tick.t)
            .into();
    }

    // this is probably the simplest way to know if a bunch of animations are done
    // since this system has a `Single` query, despawning the entity from the query
    // means this system gets disabled until another entity gets spawned, by PleaseSpawnReady
    if ready_letters_controller.all_finished() {
        commands.entity(*ready).despawn();
        commands.spawn(PleaseSpawnReady);
    }
}

impl Animation for ReadyLetter {
    fn configuration(&self) -> impl Into<AnimationConfiguration> {
        AnimationConfiguration::from(self.duration())
            // each letter has a slightly longer delay than the letter before
            .delay_by(self.delay())
            // we remove the whole tree of entities at once in `wait_for_ready`
            .remove_nothing()
    }

    fn system() -> (impl ScheduleLabel, TickSystemConfigs) {
        (Update, tick_ready_letter.into_configs())
    }
}

fn startup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: Color::BLACK.into(),
            ..default()
        },
    ));

    commands.spawn(PleaseSpawnReady);
}

#[derive(Component)]
struct PleaseSpawnReady;

impl Animation for PleaseSpawnReady {
    fn configuration(&self) -> impl Into<AnimationConfiguration> {
        AnimationConfiguration::from(0.75).despawn_entity()
    }

    fn system() -> (impl ScheduleLabel, TickSystemConfigs) {
        (
            Update,
            (|| {
                // since we're using this as a fancy timer for an observer there's actually
                // nothing to do here. in a release build i suspect this would get optimized
                // down to nothing at all but don't take my word for it, i didn't godbolt it
                // or anything
            })
            .into_configs(),
        )
    }
}

fn please_spawn_ready(_: On<AnimationFinished<PleaseSpawnReady>>, mut commands: Commands) {
    let text_font = TextFont {
        font_size: FontSize::Vw(10.0),
        ..default()
    };
    commands.spawn((
        Ready,
        Node {
            display: Display::Grid,
            width: percent(100.0),
            height: percent(100.0),
            column_gap: percent(2.0),
            grid_template_rows: vec![
                GridTrack::fr(1.0),
                GridTrack::min_content(),
                GridTrack::fr(2.0),
            ],
            grid_template_columns: vec![
                RepeatedGridTrack::auto(1),
                RepeatedGridTrack::min_content(READY.len() as u16),
                RepeatedGridTrack::auto(1),
            ],
            ..default()
        },
        Children::spawn(SpawnIter(READY.chars().enumerate().map(
            move |(i, letter)| {
                (
                    ReadyLetter(i),
                    Node {
                        grid_row: GridPlacement::start(2),
                        grid_column: GridPlacement::start(2 + i as i16),
                        ..default()
                    },
                    Text(letter.into()),
                    // just need to make sure it's invisible at the start
                    TextColor::from(Color::BLACK),
                    text_font.clone(),
                )
            },
        ))),
    ));
}
