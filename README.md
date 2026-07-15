# bevy_ecs_animations
An ECS-first approach to procedural animation in the Bevy engine, with an eye toward fine control without too much boilerplate.

## What is it?
To put it simply, this is a timeline manager that otherwise has little opinion about what you do with it. You define a `Component` type that implements the `ECSAnimation` trait, register it during app configuration, and then insert it on some entity, probably one with some components you want to animate. Each tick of the schedule you configure the system you configure will be invoked, and every entity with a component configured as an animation will have a `Tick` component inserted with the current state of the timeline so long as an animation is active.

There are some details around controlling things and some observers, but that's about the essence of things. It's a very simple abstraction that aims to formalize the plurality of ways one might tick an aniamtion around the idioms of the Bevy ECS.

The examples cover some of the ways you can use this plugin to manage animating arbitrary things.

## Features
- Component-driven animations with typed compile-time ECS access. Your animations are
  normal components, you write an ordinary system that runs in the context of the timeline
- Bevy-friendly API - control animation parameters directly from animation components,
  react by observing the entity, interact using a system param, or issue commands.
- No restrictions on what properties or types can be animated, if you want to use a curve that
  produces `(UiTransform, TextColor)` to make some text pop we are here for it. If you want
  to target arbitrary entities from one tick system, you'll have to do the work to make that happen,
  but we'll support you.
- Experimental [Curve](https://docs.rs/bevy_math/latest/bevy_math/curve/trait.Curve.html) combinators 
  with const constructors, for the efficiency fiend in you - guarantee allocation-free construction,
  or even make a complicated compile-time curve if you want to write down insane type names.

## Installation
```sh
cargo add bevy_ecs_animations
```

## Quick Start
(see [examples/basic.rs](examples/basic.rs))
```rust
use bevy::{
    prelude::*,
    ecs::schedule::ScheduleLabel,
};
use bevy_ecs_animations::*;

#[derive(Component, Default)]
struct Alpha(f32);

#[derive(Component)]
struct Fade;

fn fade(mut fades: Query<(&Tick<Fade>, &mut Alpha)>) {
    for (tick, mut alpha) in fades.iter_mut() {
       alpha.0 = tick.normalized_t;
    }
}

impl ECSAnimation for Fade {
    fn system() -> (impl ScheduleLabel, ECSAnimationConfigs) {
        (Update, fade.into_configs())
    }

    fn configuration(&self) -> impl Into<AnimationConfiguration> {
        2.5
    }
}

fn main() -> AppExit {
    App::new()
        .add_plugins(MinimalPlugins)
        .register_animation::<Fade>()
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn((Fade, Alpha::default()));
        })
        .run()
}
```

## Requirements
- Bevy 0.19
- MSRV: 1.95.0. This plugin will generally follow Bevy's lead here

### Compatibility
Generally, this plugin will track Bevy versions. Since Bevy is currently pre-1.0, this means we match minor version. Specifically:
|bevy|bevy_ecs_animations|
|-|-|
|0.19.x|0.19.x|

### Fair Warning!
Version numbers are set up to track Bevy compatibility, but much like Bevy this is under active development and there will be breaking changes as my needs evolve. Once the ecosystem crosses 1.0 there will be stronger guarantees.

## Cargo Features
This plugin has an optional experimental module that offers const combinators that work with the `Curve` trait in bevy_math. It is currently activated by default, along with the `std` backend (glam). If you wish to use them with a different backend you'll have to disable default features first then re-enable. Combinators may move to separate library altogether, or vanish. Still not sure.

|feature|description|
|-|-|
|combinators|experimental const combinators that work with Bevy's Curve trait|
|std|Activate the `std` backend feature in bevy_math (glam)|
|libm|Activate the `libm` backend feature in bevy_math|
|nostd-libm|Activate the `nostd-libm` backend feature in bevy_math|

## Why?
I wanted to procedurally animate some text, and I found the Bevy animation plugin to be a little clunky for the task. I decided to make something that uses a different approach to creating animations, leaning into the Rust type system and the Bevy ECS, in particular with strongly-typed queries and influence over scheduling, so animation ticking systems can be parallelized. This plugin focuses on opinionated timeline management, and tries to leave everything else up to the user (with reasonable defaults).

## What it doesn't do
- No glTF support, I don't use it, so I don't even know what it would take and I doubt it'll ever come up for me.
- No built-in blending. If you want to apply simultaneous animations to a single property it's up to you to decide how to make that work, at least for now.
- No direct support for animations that target multiple entities. Users are encouraged to use a component-per-animation type, instance-per-entity model. However, since tick systems can define arbitrary parameters, it is possible to make just about anything happen if you really want to (though try normal systems first!).

## Contributing
Contributions are welcome. Open an issue or submit a pull request with improvements or fixes, or look for blamelessgames on the Bevy discord.

## License
This project is free, open source and permissively licensed. Take it and do as you wish.
Except where noted (below and/or in individual files), all code in this repository is dual-licensed under either:

* MIT License ([LICENSE-MIT](LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

at your option.
This means you can select the license you prefer!
This dual-licensing approach is the de-facto standard in the Rust ecosystem and there are [very good reasons](https://github.com/bevyengine/bevy/issues/2373) to include both, at least according to bevy (and I trust them).