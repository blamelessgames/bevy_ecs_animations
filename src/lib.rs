//! ECS-driven Animations for Bevy
//!
//! ## How to use it
//!
//! ```no_run
//! use std::range::Range;
//! use bevy::{prelude::*, ecs::system::{lifetimeless::*, StaticSystemParam}};
//! use bevy_ecs_animations::*;
//!
//! // 1. define a component and implement EntityAnimation
//!
//! #[derive(Component)]
//! struct FadeIn;
//!
//! impl EntityAnimation for FadeIn {
//!     
//!     // define the param your tick function receives
//!     type Param = SQuery<Write<TextColor>, With<Self>>;
//!
//!     // define the domain your animation runs. this is in seconds
//!     // and it starts ticking when the component is inserted
//!     fn domain(&self) -> Range<f32> {
//!         (0.0..4.0).into()
//!     }
//!     
//!     // define the tick method, which will get invoked once
//!     // per frame until the domain is covered
//!     fn tick(
//!         &mut self,
//!         entity: Entity,
//!         t: f32,
//!         _dt: f32,
//!         param: &mut StaticSystemParam<Self::Param>
//!     ) {
//!         let Ok(mut color) = param.get_mut(entity) else {
//!             return;
//!         };
//!         // ease functions expect unit input, so normalize t first
//!         let t = self.normalized_t(t);
//!         let alpha = EaseFunction::CubicIn.sample_unchecked(t);
//!         color.set_alpha(alpha);
//!     }
//! }
//!
//! // 2. configure the plugin for the animation type
//! fn main() -> AppExit {
//!     App::new()
//!         .add_plugins((DefaultPlugins, EntityAnimationPlugin::<FadeIn>::default()))
//!         .add_systems(Startup, startup)
//!         .run()
//! }
//!
//! // 3. spawn an animation on an entity in a system
//! fn startup(mut commands: Commands) {
//!     commands.spawn((
//!         Camera2d::default(),
//!         Camera {
//!             clear_color: ClearColorConfig::Custom(Color::BLACK),
//!             ..default()
//!         },
//!     ));
//!     commands.spawn((
//!         // inserting the component on an entity starts the animation
//!         FadeIn,
//!         Node {
//!             width: percent(100.0),
//!             height: percent(100.0),
//!             padding: UiRect::top(percent(20.0)),
//!             ..default()
//!         },
//!         Text::from("TEXT"),
//!         TextFont {
//!             font_size: FontSize::Vw(15.0),
//!             ..default()
//!         },
//!         TextLayout::justify(Justify::Center),
//!     ));
//! }
//!
//! ```
//!
//!
//!
//! ## Rambling
//!
//! Bevy's built-in animation plugin abstracts the notion of changing properties on particular
//! components over time, according to some curve definition that can produce values given a
//! timestamp across a domain that maps to seconds by default. Because it is designed with an
//! internal animation graph that abstracts blend operations to nodes outside the property accesses,
//! there are a few limitations. Primarily, only types the system knows how to blend are supported.
//! It is not trivial to animate the hue of HSLA colors, or the translation of UI nodes, as examples.
//! The specific components need to be accessed via [EntityMut](bevy_ecs::world::EntityMut), and only
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
//! state (whatever that happens to be, I'm not the boss of users), implement the [EntityAnimation] trait
//! on it, then register the component with the [EntityAnimationPlugin] during app initialization and
//! insert an instance on entities that should be animated, optionally managing the animation via
//! commands or an [AnimationController]. The [EntityAnimation] trait centers on the
//! [tick](EntityAnimation::tick) method, which is invoked periodically while the
//! animation in question is live. The trait defines the coarse-grained management
//! behavior via associated types and function definitions, with an aim of allowing full control when
//! the (hopefully reasonable) defaults are insufficient.

// more probably makes sense here but this should hold true until it doesn't
#![deny(unsafe_code)]

#[cfg(feature = "combinators")]
pub mod combinators;
mod core;

pub use core::*;
