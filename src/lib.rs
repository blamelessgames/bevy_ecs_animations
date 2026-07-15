//! # bevy_ecs_animations
//! An ECS approach to procedural animation in the Bevy engine
//!
//! ## What is it?
//! This crate gives you a lightweight, opinionated **timeline manager** with minimal ceremony.
//! You define a normal [`Component`](bevy_ecs::component::Component) that implements the [`Animation`] trait,
//! add it to the app, and insert instances on entities you want to animate.
//!
//! While the animation is active, a [`Tick`] component is automatically added to the entity.
//! Your system reads this `Tick` to know where you are in the timeline (`t`, `normalized_t`, `dt`, etc.).
//!
//! Everything else — what you animate, how you react to progress, easing, state machines, etc. — is up to you.
//!
//!
//! ## How to use it (see examples/basic.rs)
//!
//! ```
//! use bevy::{
//!     prelude::*,
//!     ecs::schedule::ScheduleLabel,
//! };
//! use bevy_ecs_animations::*;
//!
//! #[derive(Component, Default)]
//! struct Alpha(f32);
//!
//! #[derive(Component)]
//! enum Fade {
//!     In,
//!     Out,
//! }
//!
//! fn tick_fade(mut fades: Query<(&Tick<Fade>, &mut Alpha)>) {
//!     for (tick, mut alpha) in fades.iter_mut() {
//!        alpha.0 = tick.normalized_t;
//!     }
//! }
//!
//! impl Animation for Fade {
//!     fn system() -> (impl ScheduleLabel, AnimationConfigs) {
//!         (Update, tick_fade.into_configs())
//!     }
//!
//!     fn configuration(&self) -> impl Into<AnimationConfiguration> {
//!         match *self {
//!             Fade::In => AnimationConfiguration::duration(2.5),
//!             Fade::Out => AnimationConfiguration::duration(2.5).play_in_reverse(),
//!         }
//!     }
//! }
//!
//! fn run_app() -> AppExit {
//!     App::new()
//!         .add_plugins(MinimalPlugins)
//!         .add_animation::<Fade>()
//!         .add_systems(Startup, |mut commands: Commands| {
//!             commands.spawn((Fade::In, Alpha::default()));
//!         })
//!         .run()
//! }
//! ```
//!
//! ## Rambling
//!
//! Bevy's built-in animation plugin abstracts the notion of changing properties on particular
//! components over time, according to some curve definition that can produce values given a
//! timestamp across a domain that maps to seconds by default. Because it is designed with an
//! internal animation graph that abstracts blend operations to nodes outside the property accesses,
//! there are a few limitations. Primarily, only types the system knows how to blend are supported.
//! It is not trivial to animate the hue of HSLA colors, or the translation of UI nodes, as examples.
//! The specific components need to be accessed via [`EntityMut`](bevy_ecs::world::EntityMut), and only
//! a single field of an understood type can be exposed per curve, even if you want to animate several
//! properties together.
//!
//! Also, managing these animations has a lot of bookkeeping. Users have to create animation clips,
//! add curves targeting particular fields on components to the clips (including all timeline coordination
//! if multiple properties are involved). These clips are managed in the asset system. The clip asset
//! is then used to construct an animation graph, which also needs to be managed in the asset system.
//! Next, an animation player is created that references the node ID in the graph that represents the
//! clip. The player and a component holding the graph handle have to be added to an entity that will
//! behave as a "manager" for the animation in question. Next, the targeted entities need to receive
//! components identifying this manager element. There is also a separate identification component that
//! is used to match clips to particular target entities. It's extremely flexible, but it leaves a lot
//! of fiddly ceremony in the user's hands, and the nature of the identification means mistakes often
//! result in nothing happening at all - if you're missing a necessary component on a target entity it
//! simply does not show up in the relevant query, and users are left wondering why until they notice,
//!
//! This plugin takes a more concrete approach. Users define a component that holds relevant animation
//! state (whatever that happens to be, I'm not the boss of users), implement the [`Animation`] trait
//! on it, then register the component during app initialization and insert an instance on entities that
//! should be animated, optionally managing the animation via [`AnimationCommands`] or an [`AnimationController`]. The
//! [`Animation`] trait enables the plugin to tick the given system according to configuration, and
//! in general, you're just writing a normal Bevy ECS code with no restrictions.

// more probably makes sense here but this should hold true until it doesn't
#![deny(unsafe_code)]

#[cfg(feature = "combinators")]
pub mod combinators;
mod core;

pub use core::*;
