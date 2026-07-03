# bevy-ecs-animations

An ECS-first approach to procedural animation in the Bevy engine, with an eye toward fine control without too much boilerplate (boilerplate reduction in progress).

## Features

- Component-driven animations with compile-time access to the ECS
- Curve combinators that allow for static/const construction of complex animations
- Trait/Observer/SystemParam API - control animation parameters from the animation
  component, react by observing the entity, or control via a struct that targets
  the animations you want 

## Requirements

- Bevy 0.19

## Project Structure

- `src/` - Rust source files for systems, components, and the Bevy app setup
- `Cargo.toml` - Cargo configuration and dependencies

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