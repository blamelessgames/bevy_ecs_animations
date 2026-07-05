//! Demonstrates applying the same animation to several entities, and also animates text
use std::{f32::consts::*, range::Range};

use bevy::{ecs::system::lifetimeless::*, prelude::*};
use bevy_ecs::system::StaticSystemParam;
use bevy_ecs_animations::{combinators::*, *};

fn main() -> AppExit {
    App::new()
        .add_plugins((
            DefaultPlugins,
            EntityAnimationPlugin::<ReadyLetter>::default(),
        ))
        .add_systems(Startup, startup)
        .run()
}

#[derive(Component)]
struct Ready;

const READY: &str = "READY";

#[derive(Component)]
struct ReadyLetter(usize);

impl ReadyLetter {
    const fn delay(&self) -> f32 {
        self.0 as f32 * 0.15
    }

    const fn phase_duration(&self) -> f32 {
        0.65
    }

    const fn duration(&self) -> f32 {
        self.delay() * (READY.len() - 1) as f32 + self.phase_duration() + 0.35
    }

    const fn transform_curve(&self) -> impl Curve<(Val2, Vec2)> {
        delay(
            self.delay(),
            scaled_domain(
                0.0,
                self.phase_duration(),
                zip(
                    map(fn_curve(-PI, TAU - PI, |t| f32::cos(t) * 22.0), |y| {
                        Val2::px(0.0, y)
                    }),
                    map(
                        fn_curve(-PI, TAU - PI, |t| 1.5 + f32::sin(t * 1.24) * 0.5),
                        |scale| Vec2::splat(scale),
                    ),
                ),
            ),
        )
    }

    const fn text_color_curve(&self) -> impl Curve<Hsla> {
        delay(
            self.delay(),
            scaled_domain(
                0.0,
                self.phase_duration(),
                map(
                    zip(
                        scaled_output(0.0, 360.0, EaseFunction::SmootherStepIn),
                        EaseFunction::BounceOut,
                    ),
                    |(hue, alpha)| Hsla::new(hue, 0.5, 0.5, alpha),
                ),
            ),
        )
    }
}

impl EntityAnimation for ReadyLetter {
    type Param = SQuery<(Write<UiTransform>, Write<TextColor>), With<Self>>;

    fn domain(&self) -> Range<f32> {
        // start with a little delay so the fade-in can be seen
        (-2.0..self.duration()).into()
    }

    fn remove_on_finish(&self) -> bool {
        false
    }

    fn tick(
        &mut self,
        entity: Entity,
        t: f32,
        _: f32,
        targets: &mut StaticSystemParam<Self::Param>,
    ) {
        let Ok((mut transform, mut text_color)) = targets.get_mut(entity) else {
            return;
        };
        let (translation, scale) = self.transform_curve().sample_clamped(t);
        transform.translation = translation;
        transform.scale = scale;
        text_color.0 = self.text_color_curve().sample_clamped(t).into();
    }
}

fn startup(mut commands: Commands) {
    commands.spawn((
        Camera2d::default(),
        Camera {
            clear_color: Color::BLACK.into(),
            ..default()
        },
    ));

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
