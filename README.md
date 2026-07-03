# bevy-ecs-animations

An ECS-first approach to procedural animation in the Bevy engine, with an eye toward fine control without too much boilerplate (boilerplate reduction in progress).

## Features

- Component-driven animations with typed compile-time ECS access. Your animations are
  normal components, you can use state and ticking is done mutably.
- No restrictions on what properties can be animated, if you want to use a curve that
  produces `(UiTransform, TextColor)` go for it
- Curve combinators with const constructors, for the efficiency fiend in you - no allocations,
  no runtime construction costs at all
- Trait/Observer/SystemParam API - control animation parameters from animation
  components, react by observing the entity, or interact with a system param

## Why

I wanted to prodedurally animate some text, and I found the Bevy animation plugin to be a little clunky for the task. I decided to make something focused on a different approach to creating animations, that leans into the type system and the ECS, in particular with strongly-typed queries and influence over scheduling so animation ticking systems can be parallelized (I'm not 100% on this but from what I understand Bevy ticks animation players in a system that takes mutable access to all components of all entities with animations playing, I wanted to avoid this)

## What it doesn't do
- no glTF support, I don't use it, so I don't even know what it would take and I doubt it'll ever come up for me
- no multi-entity coordination, at least not yet. there are means of syncing animation starts
  but more could be done here

## Requirements

- Bevy 0.19

## Contributing

Contributions are welcome. Open an issue or submit a pull request with improvements or fixes.

## License

This project is free, open source and permissively licensed.
Except where noted (below and/or in individual files), all code in this repository is dual-licensed under either:

* MIT License ([LICENSE-MIT](LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

at your option.
This means you can select the license you prefer!
This dual-licensing approach is the de-facto standard in the Rust ecosystem and there are [very good reasons](https://github.com/bevyengine/bevy/issues/2373) to include both, at least according to bevy (and I trust them).