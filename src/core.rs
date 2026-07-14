use std::{
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
/// fn fade(mut fades: Query<(&Tick<Fade>, &mut Alpha)>) {
///     for (tick, mut alpha) in fades.iter_mut() {
///         alpha.0 = tick.normalized_t;
///     }
/// }
///
/// impl ECSAnimation for Fade {
///     fn system() -> (impl ScheduleLabel, ECSAnimationConfigs) {
///         (Update, fade.into_configs())
///     }
///
///     fn configuration(&self) -> impl Into<AnimationConfiguration> {
///         2.5
///     }
/// }
///
/// fn setup(app: &mut App) {
///     app
///         .register_ecs_animation::<Fade>()
///         .add_systems(Startup, |mut commands: Commands| {
///             commands.spawn((Fade, Alpha::default()));
///         });
/// }
/// ```
pub trait ECSAnimation: Component + 'static {
    /// return a schedule label and your system configs to run in an animated context
    fn system() -> (impl ScheduleLabel, ECSAnimationConfigs);

    fn configuration(&self) -> impl Into<AnimationConfiguration>;
}

#[derive(Component, Debug)]
// maybe SparseSet?
#[component(immutable)]
pub struct Tick<A: ECSAnimation> {
    pub t: f32,
    pub dt: f32,
    pub normalized_t: f32,
    pub repetition: u32,
    finished: bool,
    _animation: PhantomData<fn() -> A>,
}

pub type ECSAnimationConfigs = ScheduleConfigs<ScheduleSystem>;

pub trait ECSAnimationsApp {
    fn register_ecs_animation<A: ECSAnimation>(&mut self) -> &mut Self;
}

impl ECSAnimationsApp for App {
    fn register_ecs_animation<A: ECSAnimation>(&mut self) -> &mut Self {
        self.add_plugins(ECSAnimationPlugin::<A>::default())
    }
}

/// Registers a [Component] that implements [ECSAnimation] with your app, which sets up the
/// ticking infrastructure. Most easily used as an app extension a la
/// ```
/// use bevy::{prelude::*, ecs::schedule::ScheduleLabel};
/// use bevy_ecs_animations::*;
///
/// #[derive(Component)]
/// struct Animation;
///
/// impl ECSAnimation for Animation {
///     fn system() -> (impl ScheduleLabel, ECSAnimationConfigs) {
///         (Update, (||{}).into_configs())
///     }
///
///     fn configuration(&self) -> impl Into<AnimationConfiguration> {
///         1.0
///     }
/// }
///
/// fn plugin(app: &mut App) {
///     app.register_ecs_animation::<Animation>();
/// }
///
/// ```
#[derive(Copy, Clone)]
pub struct ECSAnimationPlugin<A: ECSAnimation>(PhantomData<fn() -> A>);

impl<A: ECSAnimation> ECSAnimationPlugin<A> {
    fn ticker(
        mut animations: Query<(Entity, &mut ECSAnimationState<A>)>,
        mut commands: Commands,
        time: Res<Time>,
    ) {
        for (entity, mut state) in animations.iter_mut() {
            let Some(tick) = state.tick(time.delta_secs()) else {
                // this system can end up trying to remove this component from an animation in the same
                // frame as a despawn and the order is who knows what so do this silently
                commands
                    .entity(entity)
                    .queue_silenced(|mut entity: EntityWorldMut| {
                        entity.remove::<Tick<A>>();
                    });
                continue;
            };
            commands.entity(entity).insert(tick);
        }
    }

    fn post_ticker(
        mut animations: Query<(Entity, &mut ECSAnimationState<A>, &Tick<A>)>,
        mut commands: Commands,
    ) {
        // note! only runs for animations that actually ticked
        for (entity, mut state, tick) in animations.iter_mut() {
            if tick.finished {
                state.repetition_finished();
            }

            if state.just_repeated() && state.configuration.events {
                commands.entity(entity).trigger(ecs_animation_repeated::<A>);
            }

            if state.finished() {
                // if we're all done, removing the tick ends our involvement
                commands.entity(entity).remove::<Tick<A>>();

                if state.configuration.events {
                    commands.entity(entity).trigger(ecs_animation_finished::<A>);
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

impl<A: ECSAnimation> Default for ECSAnimationPlugin<A> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<A: ECSAnimation> Plugin for ECSAnimationPlugin<A> {
    fn build(&self, app: &mut App) {
        let (schedule, tick_system) = A::system();
        app.add_systems(
            schedule,
            (
                ECSAnimationPlugin::<A>::ticker,
                tick_system,
                ECSAnimationPlugin::<A>::post_ticker,
            )
                .chain(),
        )
        .add_observer(on_insert::<A>)
        .add_observer(on_remove::<A>)
        .register_required_components::<A, ECSAnimationState<A>>();
    }
}

fn on_insert<A: ECSAnimation>(
    add_animation: On<Insert, A>,
    mut animations: Query<(&A, &mut ECSAnimationState<A>)>,
) {
    // this serves as the initialization phase
    let Ok((animation, mut state)) = animations.get_mut(add_animation.entity) else {
        return;
    };
    state.reset(animation.configuration().into());
}

fn on_remove<A: ECSAnimation>(state: On<Remove, A>, mut commands: Commands) {
    commands
        .entity(state.entity)
        // silenced because if this was a despawn, bevy complains
        .queue_silenced(|mut entity: EntityWorldMut| {
            entity.remove::<ECSAnimationState<A>>().remove::<Tick<A>>();
        });
}

/// Observe an entity to get notified when an animation finishes
#[derive(EntityEvent)]
pub struct ECSAnimationFinished<A: ECSAnimation>(#[event_target] Entity, PhantomData<fn() -> A>);

fn ecs_animation_finished<A: ECSAnimation>(entity: Entity) -> ECSAnimationFinished<A> {
    ECSAnimationFinished(entity, PhantomData)
}

/// Observe an entity to get notified when an animation repeats
#[derive(EntityEvent)]
pub struct ECSAnimationRepeated<A: ECSAnimation>(#[event_target] Entity, PhantomData<fn() -> A>);

fn ecs_animation_repeated<A: ECSAnimation>(entity: Entity) -> ECSAnimationRepeated<A> {
    ECSAnimationRepeated(entity, PhantomData)
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
    /// this is accumulated from the tick system's contextual [Time] resource
    pub const fn elapsed(&self) -> f32 {
        self.elapsed
    }

    /// last delta time of the tick system's contextual [Time] resource
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
struct ECSAnimationState<A> {
    // the public part of the state
    state: AnimationState,
    // the elapsed timer for delays
    delay: f32,
    // if the animation just repeated
    just_repeated: bool,
    _animation: PhantomData<fn() -> A>,
}

impl<A> Deref for ECSAnimationState<A> {
    type Target = AnimationState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<A> DerefMut for ECSAnimationState<A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

impl<A> Default for ECSAnimationState<A> {
    fn default() -> Self {
        ECSAnimationState {
            state: Default::default(),
            delay: 0.0,
            just_repeated: false,
            _animation: PhantomData,
        }
    }
}

impl<A> From<&ECSAnimationState<A>> for AnimationState {
    fn from(value: &ECSAnimationState<A>) -> Self {
        value.state
    }
}

impl<A> ECSAnimationState<A> {
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
}

impl<A: ECSAnimation> ECSAnimationState<A> {
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

/// control all aspects of animations, immediately
#[derive(SystemParam)]
pub struct AnimationController<'w, 's, A: Component> {
    animations: Query<'w, 's, (&'static A, &'static mut ECSAnimationState<A>)>,
}

impl<'w, 's, A: ECSAnimation> AnimationController<'w, 's, A> {
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

/// Command interface to control animations commands-style. if you want something
/// more immediate, use AnimationController as a system parameter
pub trait ECSAnimationCommands {
    fn restart<A: ECSAnimation>(&mut self, entity: Entity) -> &mut Self;

    fn restart_all<A: ECSAnimation>(&mut self) -> &mut Self;

    fn flip_pause<A: ECSAnimation>(&mut self, entity: Entity) -> &mut Self;

    fn pause<A: ECSAnimation>(&mut self, entity: Entity) -> &mut Self;

    fn unpause<A: ECSAnimation>(&mut self, entity: Entity) -> &mut Self;

    fn flip_pause_all<A: ECSAnimation>(&mut self) -> &mut Self;

    fn pause_all<A: ECSAnimation>(&mut self) -> &mut Self;

    fn unpause_all<A: ECSAnimation>(&mut self) -> &mut Self;
}

impl ECSAnimationCommands for Commands<'_, '_> {
    fn restart<A: ECSAnimation>(&mut self, entity: Entity) -> &mut Self {
        self.queue(move |world: &mut World| {
            let mut query = world.query::<(&mut ECSAnimationState<A>, &A)>();
            let Ok((mut state, animation)) = query.get_mut(world, entity) else {
                return;
            };
            state.reset(animation.configuration().into());
        });
        self
    }

    fn restart_all<A: ECSAnimation>(&mut self) -> &mut Self {
        self.queue(|world: &mut World| {
            let mut query = world.query::<(&mut ECSAnimationState<A>, &A)>();
            for (mut state, animation) in query.iter_mut(world) {
                state.reset(animation.configuration().into());
            }
        });
        self
    }

    fn flip_pause<A: ECSAnimation>(&mut self, entity: Entity) -> &mut Self {
        self.queue(move |world: &mut World| {
            if let Some(mut state) = world.get_mut::<ECSAnimationState<A>>(entity) {
                state.paused = !state.paused;
            }
        });
        self
    }

    fn pause<A: ECSAnimation>(&mut self, entity: Entity) -> &mut Self {
        self.queue(move |world: &mut World| {
            if let Some(mut state) = world.get_mut::<ECSAnimationState<A>>(entity) {
                state.paused = true;
            }
        });
        self
    }

    fn unpause<A: ECSAnimation>(&mut self, entity: Entity) -> &mut Self {
        self.queue(move |world: &mut World| {
            if let Some(mut state) = world.get_mut::<ECSAnimationState<A>>(entity) {
                state.paused = false;
            }
        });
        self
    }

    fn flip_pause_all<A: ECSAnimation>(&mut self) -> &mut Self {
        self.queue(move |world: &mut World| {
            let mut query = world.query::<&mut ECSAnimationState<A>>();
            for mut state in query.iter_mut(world) {
                state.paused = !state.paused;
            }
        });
        self
    }

    fn pause_all<A: ECSAnimation>(&mut self) -> &mut Self {
        self.queue(move |world: &mut World| {
            let mut query = world.query::<&mut ECSAnimationState<A>>();
            for mut state in query.iter_mut(world) {
                state.paused = true;
            }
        });
        self
    }

    fn unpause_all<A: ECSAnimation>(&mut self) -> &mut Self {
        self.queue(move |world: &mut World| {
            let mut query = world.query::<&mut ECSAnimationState<A>>();
            for mut state in query.iter_mut(world) {
                state.paused = false;
            }
        });
        self
    }
}
