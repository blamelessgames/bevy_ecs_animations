# bevy-ecs-animations

An ECS-first approach to procedural animation in the Bevy engine, with an eye toward fine control without too much boilerplate.

## Features

- Component-driven animations with typed compile-time ECS access. Your animations are
  normal components, you can use state and ticking is done mutably.
- Bevy-friendly API - control animation parameters directly from animation components,
  react by observing the entity, interact using a system param, or issue commands.
- No restrictions on what properties can be animated, if you want to use a curve that
  produces `(UiTransform, TextColor)` to make some text pop we are here for it.
- Curve combinators with const constructors, for the efficiency fiend in you - guarantee
  allocation-free construction, or even make a compile-time curve if you want to write down
  insane type names.

## Installation
```sh
cargo add bevy-ecs-animations
```

## Quick Start
```rust
// examples/basic.rs
use bevy::{
    ecs::system::{StaticSystemParam, lifetimeless::*},
    prelude::*,
};
use bevy_ecs_animations::*;
use std::range::Range;

// 1. define a component and implement EntityAnimation

#[derive(Component)]
struct FadeIn;

impl EntityAnimation for FadeIn {
    // define the param your tick function receives
    type Param = SQuery<Write<TextColor>, With<Self>>;

    // define the domain your animation runs. this is in seconds
    // and it starts ticking when the component is inserted
    fn domain(&self) -> Range<f32> {
        (0.0..4.0).into()
    }

    // define the tick method, which will get invoked once
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
        // ease functions expect unit input, so normalize t first
        let t = self.normalized_t(t);
        let alpha = EaseFunction::CubicIn.sample_unchecked(t);
        color.set_alpha(alpha);
    }
}

// 2. configure the plugin for the animation type
fn main() -> AppExit {
    App::new()
        .add_plugins((
          DefaultPlugins,
          // Every type gets its own tick infrastructure, to maximize opportunities
          // to parallelize system invocations
          EntityAnimationPlugin::<FadeIn>::default()
        ))
        .add_systems(Startup, startup)
        .run()
}

// 3. spawn an animation on an entity in a system
fn startup(mut commands: Commands) {
    commands.spawn((
        Camera2d::default(),
        Camera {
            clear_color: Color::BLACK.into(),
            ..default()
        },
    ));
    commands.spawn((
        // inserting the component on an entity starts the animation
        // by default the component will be removed from the entity once the timeline is expended
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
```

## Requirements
- Bevy 0.19
- MSRV: As this plugin follows Bevy, the Minimum Supported Rust Version (MSRV) is generally close to "the latest stable release" of Rust. If it can compile Bevy, this should also work.

### Compatibility
Generally, this plugin will track Bevy versions. Since Bevy is currently pre-1.0, this means we match minor version. Specifically:
| bevy    | bevy-ecs-animations |
|---------|---------------------|
| 0.19.x  | 0.19.x              |


### Fair Warning
Version numbers are set up to track Bevy compatibility, but much like Bevy this is under active development and there will be breaking changes as my needs evolve. Once the ecosystem crosses 1.0 there will be stronger guarantees.

## Why?

I wanted to procedurally animate some text, and I found the Bevy animation plugin to be a little clunky for the task. I decided to make something focused on a different approach to creating animations, that leans into the type system and the ECS, in particular with strongly-typed queries and influence over scheduling, so animation ticking systems can be parallelized. (From what I gather Bevy ticks animation players in a system that takes mutable access to all components of all entities with animations playing, which I believe means any entity being animated is held exlusively for the duration of that system's active life)

## What it doesn't do
- no glTF support, I don't use it, so I don't even know what it would take and I doubt it'll ever come up for me
- no built-in blending. If you want to apply simultaneous animations to a single property it's up to you to decide how to make that work, at least for now.

## Contributing

Contributions are welcome. Open an issue or submit a pull request with improvements or fixes.

## License

This project is free, open source and permissively licensed. Take it and do as you wish.
Except where noted (below and/or in individual files), all code in this repository is dual-licensed under either:

* MIT License ([LICENSE-MIT](LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

at your option.
This means you can select the license you prefer!
This dual-licensing approach is the de-facto standard in the Rust ecosystem and there are [very good reasons](https://github.com/bevyengine/bevy/issues/2373) to include both, at least according to bevy (and I trust them).