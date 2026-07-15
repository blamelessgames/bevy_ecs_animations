use std::{
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use bevy_app::{App, Plugin};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::EntityEvent,
    lifecycle::{Insert, Remove},
    observer::On,
    query::Has,
    schedule::{IntoScheduleConfigs, ScheduleConfigs, ScheduleLabel},
    system::{Commands, Query, Res, ScheduleSystem, SystemParam},
    world::{EntityWorldMut, World},
};
use bevy_time::Time;

// bye bye, old
mod config;
mod deprecated;

#[cfg(test)]
mod test;

pub use config::*;
pub use deprecated::*;

/// Core trait to define a component as an animation.
/// ```
/// use bevy::{
///     prelude::*,
///     ecs::schedule::ScheduleLabel,
/// };
/// use bevy_ecs_animations::*;
///
/// #[derive(Component, Default)]
/// struct Alpha(f32);
///
/// #[derive(Component)]
/// struct Fade;
///
/// fn tick_fade(mut fades: Query<(&Tick<Fade>, &mut Alpha)>) {
///     for (tick, mut alpha) in fades.iter_mut() {
///         alpha.0 = tick.normalized_t;
///     }
/// }
///
/// impl Animation for Fade {
///     fn system() -> (impl ScheduleLabel, TickSystemConfigs) {
///         (Update, tick_fade.into_configs())
///     }
///
///     fn configuration(&self) -> impl Into<AnimationConfiguration> {
///         2.5
///     }
/// }
///
/// fn setup(app: &mut App) {
///     app
///         .add_animation::<Fade>()
///         .add_systems(Startup, |mut commands: Commands| {
///             commands.spawn((Fade, Alpha::default()));
///         });
/// }
/// ```
pub trait Animation: Component + 'static {
    /// Information for the animation's tick system. In a generic context this is a little odd but bearable:
    /// ```no_run
    /// fn system() -> (impl ScheduleLabel, TickSystemConfigs) {
    ///     (Update, tick_system.into_configs())
    /// }
    /// ```
    ///
    /// You have fairly broad control over the schedule
    /// ```no_run
    /// fn system() -> (impl ScheduleLabel, TickSystemConfigs) {
    ///     (
    ///         SomeCustomSchedule,
    ///         (
    ///             tick_system.after(some_other_system),
    ///             after_tick_system
    ///         ).chain()
    ///     )
    /// }
    /// ```
    ///
    /// Note that [`TickSystemConfigs`] is just a nice type alias to make things
    /// readable, there's nothing special about a tick system as far as this crate is concerned
    fn system() -> (impl ScheduleLabel, TickSystemConfigs);

    /// The configuration of the animation, minimally a duration.
    /// As [`AnimationConfiguration`] implements `From<f32>` you can just
    /// return that if it's all you need
    /// ```no_run
    /// fn configuration(&self) -> impl Into<AnimationConfiguration> {
    ///     2.0
    /// }
    /// ```
    ///
    /// This is invoked whenever an animation is inserted on an entity, and the returned
    /// state is used for the duration of that animation instance
    fn configuration(&self) -> impl Into<AnimationConfiguration>;
}

/// Information about the current state of an active [`Animation`], inserted as a component
/// on the same entity while the animation is active (i.e. started, not paused, not finished)
#[derive(Component)]
// maybe SparseSet?
#[component(immutable)]
pub struct Tick<A: Animation> {
    pub t: f32,
    pub dt: f32,
    pub normalized_t: f32,
    pub repetition: u32,
    finished: bool,
    _animation: PhantomData<fn() -> A>,
}

impl<A: Animation> Debug for Tick<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tick")
            .field("t", &self.t)
            .field("dt", &self.dt)
            .field("normalized_t", &self.normalized_t)
            .field("repetition", &self.repetition)
            .finish()
    }
}

pub type TickSystemConfigs = ScheduleConfigs<ScheduleSystem>;

/// Extends [`App`] with a registration function for [`Animation`]s, enabling the plugin
/// to maintain a timeline for the system processing ticks
pub trait AnimationAppExt {
    /// Registers a [`Component`] that implements [`Animation`] with your app, which sets up the
    /// ticking infrastructure
    /// ```
    /// use bevy::{prelude::*, ecs::schedule::ScheduleLabel};
    /// use bevy_ecs_animations::*;
    ///
    /// #[derive(Component)]
    /// struct YourAnimation;
    ///
    /// fn tick_animation(_:Query<&Tick<Animation>>) {
    ///     // animate something. anything. everything!
    /// }
    ///
    /// impl Animation for YourAnimation {
    ///     fn system() -> (impl ScheduleLabel, TickSystemConfigs) {
    ///         (Update, tick_animation.into_configs())
    ///     }
    ///
    ///     fn configuration(&self) -> impl Into<AnimationConfiguration> {
    ///         1.0 // second
    ///     }
    /// }
    ///
    /// fn plugin(app: &mut App) {
    ///     // this makes the magic happen
    ///     app.add_animation::<Animation>();
    /// }
    ///
    /// ```
    fn add_animation<A: Animation>(&mut self) -> &mut Self;
}

impl AnimationAppExt for App {
    fn add_animation<A: Animation>(&mut self) -> &mut Self {
        self.add_plugins(AnimationPlugin::<A>::default())
    }
}

#[derive(Copy, Clone)]
struct AnimationPlugin<A: Animation>(PhantomData<fn() -> A>);

impl<A: Animation> AnimationPlugin<A> {
    fn ticker(
        mut animations: Query<(Entity, &mut InternalAnimationState<A>, Has<Tick<A>>)>,
        mut commands: Commands,
        time: Res<Time>,
    ) {
        for (entity, mut state, has_tick) in animations.iter_mut() {
            if let Some(tick) = state.tick(time.delta_secs()) {
                commands.entity(entity).insert(tick);
                continue;
            };

            if has_tick {
                // this system can end up trying to remove this component from an animation in the same
                // frame as a despawn and the order is who knows what so we do this silently
                commands
                    .entity(entity)
                    .queue_silenced(|mut entity: EntityWorldMut| {
                        entity.remove::<Tick<A>>();
                    });
            }
        }
    }

    fn post_tick(
        mut animations: Query<(Entity, &mut InternalAnimationState<A>, &Tick<A>)>,
        mut commands: Commands,
    ) {
        // note! only runs for animations that actually ticked
        for (entity, mut state, tick) in animations.iter_mut() {
            if tick.finished {
                state.repetition_finished();
            }

            if state.just_repeated() && state.configuration.events {
                commands.entity(entity).trigger(animation_repeated::<A>);
            }

            if state.finished() {
                // if we're all done, removing the tick ends our involvement
                commands.entity(entity).remove::<Tick<A>>();

                if state.configuration.events {
                    commands.entity(entity).trigger(animation_finished::<A>);
                }
                // we aren't quiet about these moves because they're in the public API,
                // so we should warn on things going wrong since that's the user asking
                // for the wrong thing
                match state.configuration.removal {
                    RemovalOptions::Component => {
                        commands.entity(entity).remove::<A>();
                    }
                    RemovalOptions::Entity => {
                        commands.entity(entity).despawn();
                    }
                    RemovalOptions::Nothing => { /* just so */ }
                }
            }
        }
    }
}

impl<A: Animation> Default for AnimationPlugin<A> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<A: Animation> Plugin for AnimationPlugin<A> {
    fn build(&self, app: &mut App) {
        let (schedule, tick_system) = A::system();
        app.add_systems(
            schedule,
            (
                AnimationPlugin::<A>::ticker,
                tick_system,
                AnimationPlugin::<A>::post_tick,
            )
                .chain(),
        )
        .add_observer(on_insert::<A>)
        .add_observer(on_remove::<A>)
        .register_required_components::<A, InternalAnimationState<A>>();
    }
}

fn on_insert<A: Animation>(
    add_animation: On<Insert, A>,
    mut animations: Query<(&A, &mut InternalAnimationState<A>)>,
) {
    // this serves as the initialization phase
    let Ok((animation, mut state)) = animations.get_mut(add_animation.entity) else {
        return;
    };
    state.reset(animation.configuration().into());
}

fn on_remove<A: Animation>(state: On<Remove, A>, mut commands: Commands) {
    commands
        .entity(state.entity)
        // silenced because if this was a despawn, bevy complains
        .queue_silenced(|mut entity: EntityWorldMut| {
            entity
                .remove::<InternalAnimationState<A>>()
                .remove::<Tick<A>>();
        });
}

/// Observe an entity to get notified when an animation finishes
#[derive(EntityEvent)]
pub struct AnimationFinished<A: Animation>(#[event_target] Entity, PhantomData<fn() -> A>);

fn animation_finished<A: Animation>(entity: Entity) -> AnimationFinished<A> {
    AnimationFinished(entity, PhantomData)
}

/// Observe an entity to get notified when an animation repeats
#[derive(EntityEvent)]
pub struct AnimationRepeated<A: Animation>(#[event_target] Entity, PhantomData<fn() -> A>);

fn animation_repeated<A: Animation>(entity: Entity) -> AnimationRepeated<A> {
    AnimationRepeated(entity, PhantomData)
}

/// The state of the animation. Users are free to inspect it!
#[derive(Debug, Clone, Copy)]
pub struct AnimationState {
    elapsed: f32,
    last_dt: f32,
    repetitions_remaining: u32,
    finished: bool,
    paused: bool,
    configuration: AnimationConfiguration,
}

impl Default for AnimationState {
    fn default() -> Self {
        Self {
            elapsed: 0.0,
            last_dt: 0.0,
            repetitions_remaining: 0,
            finished: false,
            paused: false,
            configuration: (0.0).into(),
        }
    }
}

impl AnimationState {
    fn repetition_finished(&mut self) {
        self.repetitions_remaining -= 1;
        if self.repetitions_remaining == 0 {
            self.finished = true;
        } else {
            self.elapsed = 0.0;
            self.last_dt = 0.0;
        }
    }

    /// elapsed time. this always counts up even when the animation counts down
    /// this is accumulated from the tick system's contextual [`Time`] resource
    pub const fn elapsed(&self) -> f32 {
        self.elapsed
    }

    /// last delta time of the tick system's contextual [`Time`] resource
    pub const fn last_dt(&self) -> f32 {
        self.last_dt
    }

    pub const fn repetitions(&self) -> u32 {
        self.configuration.repetitions
    }

    pub const fn repetitions_remaining(&self) -> u32 {
        self.repetitions_remaining
    }

    pub const fn repetition(&self) -> u32 {
        (self.configuration.repetitions - self.repetitions_remaining) + 1
    }

    pub const fn finished(&self) -> bool {
        self.finished
    }

    pub const fn paused(&self) -> bool {
        self.paused
    }
}

#[derive(Component, Clone, Copy)]
struct InternalAnimationState<A> {
    // the public part of the state
    state: AnimationState,
    // the elapsed timer for delays
    delay: f32,
    // if the animation just repeated
    just_repeated: bool,
    _animation: PhantomData<fn() -> A>,
}

impl<A> Deref for InternalAnimationState<A> {
    type Target = AnimationState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<A> DerefMut for InternalAnimationState<A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

impl<A> Default for InternalAnimationState<A> {
    fn default() -> Self {
        InternalAnimationState {
            state: Default::default(),
            delay: 0.0,
            just_repeated: false,
            _animation: PhantomData,
        }
    }
}

impl<A> From<&InternalAnimationState<A>> for AnimationState {
    fn from(value: &InternalAnimationState<A>) -> Self {
        value.state
    }
}

impl<A> InternalAnimationState<A> {
    fn finished(&self) -> bool {
        self.state.finished
    }

    fn just_repeated(&mut self) -> bool {
        let just_repeated = self.just_repeated;
        self.just_repeated = false;
        just_repeated
    }

    fn reset(&mut self, configuration: AnimationConfiguration) {
        self.delay = 0.0;
        self.just_repeated = false;
        self.state = AnimationState {
            elapsed: 0.0,
            last_dt: 0.0,
            repetitions_remaining: configuration.repetitions,
            finished: configuration.repetitions == 0,
            paused: configuration.paused,
            configuration,
        };
    }

    fn finish(&mut self) {
        self.state.elapsed = (self.state.configuration.duration + self.state.configuration.start)
            - (self.state.last_dt * 0.5);
        self.state.repetitions_remaining = 0;
    }
}

impl<A: Animation> InternalAnimationState<A> {
    #[inline]
    fn normalized_t(&self, t: f32) -> f32 {
        let start = self.state.configuration.start;
        let end = start + self.state.configuration.duration;

        if start == end {
            0.0
        } else {
            (t - start) / (end - start)
        }
    }

    fn tick(&mut self, dt: f32) -> Option<Tick<A>> {
        // paused and finished animations do nothing
        if self.state.paused || self.state.finished {
            return None;
        }

        // first, treat delay as being entirely outside the timeline
        // return until we've sunk it all, be sure to account for leftover dt
        // this behavior may become configurable
        let delay = self.configuration.delay;
        let mut leftover_dt = 0.0;
        if self.delay < delay {
            self.delay += dt;
            if self.delay >= delay {
                leftover_dt += delay - self.delay;
            } else {
                return None;
            }
        }
        let dt = dt + leftover_dt;

        // now we get to  ask configuration for what really happened!
        let Some((t, dt, finished)) = self.state.configuration.t(self.state.elapsed, dt) else {
            // if None, we're outside the ticking range, just accumulate
            self.state.last_dt = dt;
            self.state.elapsed += dt;
            return None;
        };
        // otherwise accumulate what might be trimmed
        self.state.last_dt = dt;
        self.state.elapsed += dt;

        let normalized_t = self.normalized_t(t);

        Some(Tick {
            t,
            dt,
            normalized_t,
            repetition: self.state.repetition(),
            finished,
            _animation: PhantomData,
        })
    }
}

/// Query and control [`Animation`]s
#[derive(SystemParam)]
pub struct AnimationController<'w, 's, A: Component> {
    animations: Query<'w, 's, (&'static A, &'static mut InternalAnimationState<A>)>,
}

impl<'w, 's, A: Animation> AnimationController<'w, 's, A> {
    /// Reset the specified animation
    pub fn reset(&mut self, entity: Entity) -> &mut Self {
        if let Ok((animation, mut state)) = self.animations.get_mut(entity) {
            state.reset(animation.configuration().into());
        }
        self
    }

    pub fn reset_all(&mut self) -> &mut Self {
        for (animation, mut state) in self.animations.iter_mut() {
            state.reset(animation.configuration().into());
        }
        self
    }

    pub fn finish(&mut self, entity: Entity) -> &mut Self {
        if let Ok((_, mut state)) = self.animations.get_mut(entity) {
            state.finish();
        }
        self
    }

    pub fn finish_all(&mut self) -> &mut Self {
        for (_, mut state) in self.animations.iter_mut() {
            state.finish();
        }
        self
    }
}

impl<'w, 's, A: Component> AnimationController<'w, 's, A> {
    pub fn finished(&self, entity: Entity) -> Option<bool> {
        self.animations
            .get(entity)
            .ok()
            .map(|(_, state)| state.finished)
    }

    pub fn all_finished(&self) -> bool {
        self.animations.iter().all(|(_, state)| state.finished)
    }

    pub fn state(&mut self, entity: Entity) -> Option<AnimationState> {
        self.animations
            .get(entity)
            .ok()
            .map(|(_, state)| state.into())
    }

    pub fn flip_pause(&mut self, entity: Entity) -> &mut Self {
        if let Ok((_, mut state)) = self.animations.get_mut(entity) {
            state.paused = !state.paused;
        }
        self
    }

    pub fn pause(&mut self, entity: Entity) -> &mut Self {
        if let Ok((_, mut state)) = self.animations.get_mut(entity) {
            state.paused = true;
        }
        self
    }

    pub fn unpause(&mut self, entity: Entity) -> &mut Self {
        if let Ok((_, mut state)) = self.animations.get_mut(entity) {
            state.paused = false;
        }
        self
    }

    pub fn flip_pause_all(&mut self) -> &mut Self {
        for (_, mut state) in self.animations.iter_mut() {
            state.paused = !state.paused;
        }
        self
    }

    pub fn pause_all(&mut self) -> &mut Self {
        for (_, mut state) in self.animations.iter_mut() {
            state.paused = true;
        }
        self
    }

    pub fn unpause_all(&mut self) -> &mut Self {
        for (_, mut state) in self.animations.iter_mut() {
            state.paused = false;
        }
        self
    }
}

/// Command interface to control animations commands-style. if you want to make immediate
/// changes or query internal state live, use [`AnimationController`] as a system parameter
pub trait AnimationCommandsExt {
    /// Reset the tick state of the identified entity for the specified animation type
    fn reset<A: Animation>(&mut self, entity: Entity) -> &mut Self;

    /// Reset the tick states of all animations of the specified type
    fn reset_all<A: Animation>(&mut self) -> &mut Self;

    /// Pushes the tick state of the idenfitied entity of the specified type to the last frame
    /// of the last repetition.
    ///
    /// This is subject to tick system timing glitches, so might not land on exactly the last frame
    fn finish<A: Animation>(&mut self, entity: Entity) -> &mut Self;

    /// Pushes the tick state of all animations of the specified type to the last frame of the
    /// last repetition.
    ///
    /// This is subject to tick system timing glitches, so might not land on exactly the last frame
    fn finish_all<A: Animation>(&mut self) -> &mut Self;

    /// Flip the pause state of the identified entity for the specified animation type
    fn flip_pause<A: Animation>(&mut self, entity: Entity) -> &mut Self;

    /// Pause the identified entity for the specified animation type
    fn pause<A: Animation>(&mut self, entity: Entity) -> &mut Self;

    /// Unpause the identified entity for the specified animation type
    fn unpause<A: Animation>(&mut self, entity: Entity) -> &mut Self;

    /// Flip the pause states of all animations of the specified type
    fn flip_pause_all<A: Animation>(&mut self) -> &mut Self;

    /// Pause all animations of the specified type
    fn pause_all<A: Animation>(&mut self) -> &mut Self;

    /// Unpause all animations of the specified type
    fn unpause_all<A: Animation>(&mut self) -> &mut Self;
}

impl AnimationCommandsExt for Commands<'_, '_> {
    fn reset<A: Animation>(&mut self, entity: Entity) -> &mut Self {
        self.queue(move |world: &mut World| {
            let mut query = world.query::<(&mut InternalAnimationState<A>, &A)>();
            let Ok((mut state, animation)) = query.get_mut(world, entity) else {
                return;
            };
            state.reset(animation.configuration().into());
        });
        self
    }

    fn reset_all<A: Animation>(&mut self) -> &mut Self {
        self.queue(|world: &mut World| {
            let mut query = world.query::<(&mut InternalAnimationState<A>, &A)>();
            for (mut state, animation) in query.iter_mut(world) {
                state.reset(animation.configuration().into());
            }
        });
        self
    }

    fn finish<A: Animation>(&mut self, entity: Entity) -> &mut Self {
        self.queue(move |world: &mut World| {
            let mut query = world.query::<&mut InternalAnimationState<A>>();
            let Ok(mut state) = query.get_mut(world, entity) else {
                return;
            };
            state.finish();
        });
        self
    }

    fn finish_all<A: Animation>(&mut self) -> &mut Self {
        self.queue(|world: &mut World| {
            let mut query = world.query::<&mut InternalAnimationState<A>>();
            for mut state in query.iter_mut(world) {
                state.finish();
            }
        });
        self
    }

    fn flip_pause<A: Animation>(&mut self, entity: Entity) -> &mut Self {
        self.queue(move |world: &mut World| {
            if let Some(mut state) = world.get_mut::<InternalAnimationState<A>>(entity) {
                state.paused = !state.paused;
            }
        });
        self
    }

    fn pause<A: Animation>(&mut self, entity: Entity) -> &mut Self {
        self.queue(move |world: &mut World| {
            if let Some(mut state) = world.get_mut::<InternalAnimationState<A>>(entity) {
                state.paused = true;
            }
        });
        self
    }

    fn unpause<A: Animation>(&mut self, entity: Entity) -> &mut Self {
        self.queue(move |world: &mut World| {
            if let Some(mut state) = world.get_mut::<InternalAnimationState<A>>(entity) {
                state.paused = false;
            }
        });
        self
    }

    fn flip_pause_all<A: Animation>(&mut self) -> &mut Self {
        self.queue(move |world: &mut World| {
            let mut query = world.query::<&mut InternalAnimationState<A>>();
            for mut state in query.iter_mut(world) {
                state.paused = !state.paused;
            }
        });
        self
    }

    fn pause_all<A: Animation>(&mut self) -> &mut Self {
        self.queue(move |world: &mut World| {
            let mut query = world.query::<&mut InternalAnimationState<A>>();
            for mut state in query.iter_mut(world) {
                state.paused = true;
            }
        });
        self
    }

    fn unpause_all<A: Animation>(&mut self) -> &mut Self {
        self.queue(move |world: &mut World| {
            let mut query = world.query::<&mut InternalAnimationState<A>>();
            for mut state in query.iter_mut(world) {
                state.paused = false;
            }
        });
        self
    }
}
