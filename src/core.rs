use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use bevy_app::{App, Plugin, Update};
use bevy_ecs::{
    component::{Component, Mutable},
    entity::Entity,
    event::EntityEvent,
    lifecycle::{Insert, Remove},
    observer::On,
    schedule::{IntoScheduleConfigs, ScheduleLabel},
    system::{
        Commands, Local, Query, Res, Single, StaticSystemParam, SystemParam, lifetimeless::*,
    },
    world::{EntityWorldMut, World},
};
use bevy_time::Time;

/// Core trait to define a component as an animation.
///
/// When a registered component type is added to an entity, this plugin configures internal
/// state based on the values returned from trait methods and then invokes the
/// [tick](EntityAnimation::tick) method according to the specifications. That's really about it.
/// You don't even have to "animate" anything if you don't want to (but a regular system is
/// probably simpler for whatever your goal is in that case).
///
/// The semantics of the [Param](EntityAnimation::Param) type bear some explanation. First, everything
/// useful you might include here has lifetime parameters you cannot satisfy generically. The solution
/// is to use the helpers in [lifetimeless](bevy_ecs::system::lifetimeless) or the additional helpers
/// defined in this module ([SAnimationController] in particular, also [SLocal] can be helpful). Also,
/// since the surrounding system that invokes the [tick](EntityAnimation::tick) method has to follow
/// normal rust ownership semantics, it cannot pass ownership of this parameter along, because it is
/// responsible to tick every component of the same type (if any), so you only get a mutable reference.
/// There are helper methods to normalize everything so that it looks like a normal system, but the
/// differences are a little grating until you get past them.
///
/// The upshot is that you can query whatever state you like, pull in resources, issue commands - it's
/// ultimately a normal system with a complex schedule and unusual invocation semantics that are
/// well-suited to sampling some source of animation parameters and setting them where they can have
/// impacts on the game world.
pub trait EntityAnimation: Component<Mutability = Mutable> {
    /// Define the [SystemParam] for the [tick](EntityAnimation::tick) method.
    ///
    /// ```ignore
    /// type Param: (
    ///     SQuery(Write(Transform), With<Self>),
    ///     SQuery(Read(Transform), Without<Self>),
    ///     SCommands,
    ///     SLocal<usize>,
    /// );
    /// ```
    ///
    /// Two things
    /// 1. do not include the `Self` component in the query ([Self::tick] is `&mut self` so no
    ///    need internally, and there is another means of cross-instance communication) except
    ///    as a filter.
    /// 2. do not use the [AnimationController] system param for the same type (it will
    ///    conflict in the system that invokes tick and you will crash with an error in
    ///    plugin code). Controllers for other types work just fine.
    ///
    /// If you're trying to communicate across component instances you can use [SLocal]. The
    /// system that invokes component instances experiences one invocation per type per frame,
    /// so the semantics of [SLocal] are slightly different. It is shared across all instances
    /// of a given component no matter what entity they're on. if you want to control instances
    /// of other component types, then the appropriate [AnimationController] will work perfectly
    /// (and you'll probably want to use [EntityAnimationPlugin::did_tick] for ordering.
    ///
    /// Aside from that, it's what you're used to from Bevy's ECS.
    type Param: SystemParam;

    /// [ScheduleLabel] to run the tick system. Defaults to [Update].
    ///
    /// Changes to this have no effect after initialization, so it doesn't take `&self`
    fn schedule() -> impl ScheduleLabel {
        // i think i want to
        // 1. move this to the plugin for smarter semantics
        // 2. figure out how to offer even more control, like system sets or conditions
        Update
    }

    /// Returns the configuration for the animation.
    fn configuration(&self) -> impl Into<AnimationConfiguration>;

    /// Called every invocation of the schedule that an instance is active
    ///
    /// - `entity` - the entity holding this component
    /// - `t` - total seconds this animation has been running
    /// - `dt` - delta time from last invocation
    /// - `param` - [Self::Param]
    fn tick(&mut self, entity: Entity, t: f32, dt: f32, param: &mut StaticSystemParam<Self::Param>);

    /// bring `t` into [0, 1] over the current domain of the animation
    /// taken from configuration. Note that if you make updates to the configuration while an animation
    /// is running, this might return nonsensical values
    fn normalized_t(&self, t: f32) -> f32 {
        let configuration = self.configuration().into();
        let start = configuration.start;
        let end = start + configuration.duration;

        if start == end {
            0.0
        } else {
            (t - start) / (end - start)
        }
    }
}

/// Encapsulates the configuration for an animation. Construct using the [From]
/// implementation for f32, which supplies the duration, the minimal requirement.
///
/// ```
/// # use bevy_ecs_animations::AnimationConfiguration;
/// AnimationConfiguration::from(4.0); // 4 seconds. must not be negative!
/// ```
///
/// Most of this is directed toward configuring a timeline for the animation.
/// The simplest case is just supplying a duration. Your animation will run from `0.0->duration`,
/// your [tick](EntityAnimation::tick) method will get t values over that range, then the plugin will
/// trigger a finished event, remove the animation (and associated state) from the entity, and that's that.
///
/// You can do a lot more.
///
/// The animation usually starts when the timeline starts, at 0.0 seconds. You can set a different
/// start time if you need one. The animation will begin receiving ticks when the timeline
/// reaches this start time, and it will continue ticking for `duration` seconds, so your
/// tick method will receive t values running from `start->duration + start`
/// ```
/// # use bevy_ecs_animations::AnimationConfiguration;
/// AnimationConfiguration::from(4.0).start_at(2.0);
/// ```
/// You can also delay the timeline. The delay counts down before the timeline starts. Nothing about your ticks
/// changes, except the plugin doesn't start your timeline for `delay` seconds.
/// ```
/// # use bevy_ecs_animations::AnimationConfiguration;
/// AnimationConfiguration::from(4.0).delay_by(2.0);
/// ```
/// So by default, the timeline starts immediately, sending ticks to your tick method,
/// and it gets t values running from `0.0->duration`, and you can move this around.
///
/// So far the animation has always run forward, but you can also run in reverse.
/// ```
/// # use bevy_ecs_animations::AnimationConfiguration;
/// AnimationConfiguration::from(4.0).play_in_reverse();
/// ```
/// This bears some explanation. Your tick method will now receive t values in reverse order,
/// but everything else proceeds forward (time's arrow is uncompromising on this). What this means is first,
/// any delay counts down, then the timeline ticks silently for `start` seconds, then your tick method will
/// be invoked, receiving t values from `start + duration->start`.
///
/// By default, animations have 1 repetition, but you can ask for more.
/// ```
/// # use bevy_ecs_animations::AnimationConfiguration;
/// AnimationConfiguration::from(4.0).repeat(2); // or more!
/// ```
/// This is pretty straightforward - after the delay expires, the timeline is run for as many repetitions as you configure
///
/// You can also set a count of 0, which means the animation will not play until you intervene somehow. However, if you simply
/// want to start the animation paused, you can configure that.
/// ```
/// # use bevy_ecs_animations::AnimationConfiguration;
/// AnimationConfiguration::from(4.0).start_paused(true);
/// ```
/// In this case, you'll have to use a [command](AnimationCommands) or [AnimationController] to unpause it before anything animates.
///
/// You can also configure how the plugin behaves when an animation finishes, which means that the animation has
/// run through its entire timeline as many times as it has been configured to repeat. By default, the plugin will trigger
/// [EntityAnimationRepeated] when an animation finishes a repetition, and [EntityAnimationFinished] when the overall animation
/// is finished. You can disable this, if you don't care about the events (and maybe have so many animations running the overhead matters).
/// ```
/// # use bevy_ecs_animations::AnimationConfiguration;
/// AnimationConfiguration::from(4.0).trigger_events(false);
/// ```
///
/// The plugin will also do a little cleanup for you. By default, the animation component (and associated internal state) will be
/// removed from the entity on finish. This causes an archetype move and also means you cannot reset the animation or read the state.
/// If you prefer, you can have the plugin do nothing (which means it can be restarted via [commands](AnimationCommands) or an
/// [AnimationController])
/// ```
/// # use bevy_ecs_animations::AnimationConfiguration;
/// AnimationConfiguration::from(4.0).remove_nothing();
/// ```
///
/// You can also have the plugin despawn the containing entity
/// ```
/// # use bevy_ecs_animations::AnimationConfiguration;
/// AnimationConfiguration::from(4.0).despawn_entity();
/// ```
///
/// If for some reason you want the default back, it's here
/// ```
/// # use bevy_ecs_animations::AnimationConfiguration;
/// AnimationConfiguration::from(4.0).remove_animation();
/// ```
///
/// That's it so far!
#[derive(Debug, Copy, Clone)]
pub struct AnimationConfiguration {
    duration: f32,
    start: f32,
    delay: f32,
    mode: PlaybackMode,
    repetitions: u32,
    paused: bool,
    events: bool,
    removal: RemovalOptions,
}

impl From<f32> for AnimationConfiguration {
    fn from(duration: f32) -> Self {
        debug_assert!(duration >= 0.0, "negative durations cannot be set");
        Self {
            duration,
            start: 0.0,
            delay: 0.0,
            mode: Default::default(),
            repetitions: 1,
            paused: false,
            events: true,
            removal: Default::default(),
        }
    }
}

#[test]
fn test_timekeeping() {
    use float_eq::assert_float_eq;
    fn test(c: AnimationConfiguration, elapsed: f32, dt: f32, expected: Option<(f32, f32, bool)>) {
        let Some((elapsed, dt, done)) = c.t(elapsed, dt) else {
            assert_eq!(expected, None);
            return;
        };
        let (e_elapsed, e_dt, e_done) = expected.expect("expected None!");
        // millisecond precision seems fine?
        assert_float_eq!(e_elapsed, elapsed, abs <= 0.001);
        assert_float_eq!(e_dt, dt, abs <= 0.001);
        assert_eq!(e_done, done);
    }

    let c = AnimationConfiguration::from(4.0);
    test(c, 0.0, 0.001102375, Some((0.0, 0.001102375, false)));
    test(c, 0.1, 0.008, Some((0.1, 0.008, false)));
    test(c, 0.2, 0.008, Some((0.2, 0.008, false)));
    // compensate for dt going past the end
    test(c, 3.9, 1.0, Some((3.9, 0.1, true)));
    // this doesn't handle being called after finish

    let c = c.start_at(1.0);
    // it should be None the whole first second
    test(c, 0.0, 0.001102375, None);
    test(c, 0.4, 0.001102375, None);
    test(c, 0.9, 0.001102375, None);
    // exactly at start should work
    test(c, 1.0, 0.008, Some((1.0, 0.008, false)));
    // make sure we compensate for dt striding the start
    test(c, 1.1, 1.0, Some((1.1, 0.1, false)));
    test(c, 3.9, 1.0, Some((3.9, 1.0, false)));
    // the end is 1 second later now
    test(c, 4.9, 1.0, Some((4.9, 0.1, true)));

    // okay that works... now reverse it! which is mostly the same,
    // but t values run the other way
    let c = c.play_in_reverse();
    // should still not care during the first second
    test(c, 0.0, 0.001102375, None);
    test(c, 0.4, 0.001102375, None);
    test(c, 0.9, 0.001102375, None);
    // exactly at start should report back the end
    test(c, 1.0, 0.008, Some((5.0, 0.008, false)));
    // make sure we compensate for dt striding the start, but the other way
    test(c, 1.1, 1.0, Some((4.9, 0.1, false)));
    // and so it goes
    test(c, 3.9, 1.0, Some((2.1, 1.0, false)));
    test(c, 4.9, 1.0, Some((1.0, 0.1, true)));
}

impl AnimationConfiguration {
    /// handles the logic of turning last frame's elapsed time + this frame's dt into t and dt values
    /// for the animation's tick method, and also determines if it finished, or if there's no reason
    /// to tick
    fn t(&self, elapsed: f32, dt: f32) -> Option<(f32, f32, bool)> {
        let start = self.start;
        let end = start + self.duration;
        let now = elapsed + dt;
        match self.mode {
            PlaybackMode::Forward => {
                // if it's over, say so
                if now >= end {
                    // adjust the dt
                    Some((elapsed, (end - elapsed), true))
                // if its still before the start, say so
                } else if elapsed < start {
                    None
                // if we just started then adjust dt. have to be careful
                // at the boundary, if elapsed somehow is exactly start
                // (or start is 0.0) just send the dt we get back
                } else if elapsed - start > 0.0 && elapsed - start < dt {
                    Some((elapsed, elapsed - start, false))
                // actually everything is great!
                } else {
                    Some((elapsed, dt, false))
                }
            }
            // this is basically the same thing, but we play with
            // the output
            PlaybackMode::Reverse => {
                // if it's over, say so
                if now >= end {
                    Some((start, (end - elapsed), true))
                // if its still before the start, say so
                } else if elapsed < start {
                    None
                // if we just started then were are elasped - start under end. sneaky
                } else if elapsed - start > 0.0 && elapsed - start < dt {
                    let dt = elapsed - start;
                    Some((end - dt, dt, false))
                // actually everything is great!
                } else {
                    Some(((end - elapsed) + start, dt, false))
                }
            }
        }
    }
}

impl AnimationConfiguration {
    /// Set a start point on the animation's timeline when this animation begins ticking, in seconds.
    ///
    /// Defaults to 0.0
    ///
    /// Must not be negative.
    pub fn start_at(mut self, start: f32) -> Self {
        debug_assert!(start >= 0.0, "start cannot be negative");
        self.start = start;
        self
    }

    /// Seconds to "tick off" the timeline before it officially starts.
    ///
    pub fn delay_by(mut self, delay: f32) -> Self {
        debug_assert!(delay >= 0.0, "delay cannot be negative");
        self.delay = delay;
        self
    }

    /// Play the animation in forward order
    ///
    /// This is the default
    pub fn play_forward(mut self) -> Self {
        self.mode = PlaybackMode::Forward;
        self
    }

    /// Play the animation in reverse
    pub fn play_in_reverse(mut self) -> Self {
        self.mode = PlaybackMode::Reverse;
        self
    }

    /// set repetition count. 0 is valid but results in an animation that finishes
    /// immediately
    pub fn repeat(mut self, repetitions: u32) -> Self {
        self.repetitions = repetitions;
        self
    }

    /// pass `true` to pause at start, doing nothing until you intervene
    ///
    /// default is `false`
    pub fn start_paused(mut self, paused: bool) -> Self {
        self.paused = paused;
        self
    }

    /// pass `false` to disable triggering documented events
    ///
    /// default is `true`
    pub fn trigger_events(mut self, events: bool) -> Self {
        self.events = events;
        self
    }

    /// When the animation is finished, remove the component + internal state from the entity
    ///
    /// this is the default.
    pub fn remove_animation(mut self) -> Self {
        self.removal = RemovalOptions::Component;
        self
    }

    /// When the animation is finished, despawn the entity
    pub fn despawn_entity(mut self) -> Self {
        self.removal = RemovalOptions::Entity;
        self
    }

    /// When the animation is finished, leave everything sitting there. Animations can be
    /// restarted with (commands)[AnimationCommands] or an [AnimationController]
    pub fn remove_nothing(mut self) -> Self {
        self.removal = RemovalOptions::Nothing;
        self
    }
}

#[derive(Debug, Copy, Clone, Default)]
enum PlaybackMode {
    #[default]
    Forward,
    Reverse,
}

#[derive(Debug, Copy, Clone, Default)]
enum RemovalOptions {
    #[default]
    Component,
    Entity,
    Nothing,
}

/// a [Local] system param with 'static lifetime
pub type SLocal<T> = Local<'static, T>;

/// a terribly named [Single] with 'static lifetime
pub type SSingle<D, F> = Single<'static, 'static, D, F>;

/// Command interface to control animations commands-style. if you want something
/// more immediate, use AnimationController as a system parameter
pub trait AnimationCommands {
    fn restart<A: EntityAnimation>(&mut self, entity: Entity) -> &mut Self;

    fn restart_all<A: EntityAnimation>(&mut self) -> &mut Self;

    fn flip_pause<A: EntityAnimation>(&mut self, entity: Entity) -> &mut Self;

    fn pause<A: EntityAnimation>(&mut self, entity: Entity) -> &mut Self;

    fn unpause<A: EntityAnimation>(&mut self, entity: Entity) -> &mut Self;

    fn flip_pause_all<A: EntityAnimation>(&mut self) -> &mut Self;

    fn pause_all<A: EntityAnimation>(&mut self) -> &mut Self;

    fn unpause_all<A: EntityAnimation>(&mut self) -> &mut Self;
}

/// Add this an instance of this plugin for each animation type you build.
///
/// # Type Parameters
///
/// * `A` — The [EntityAnimation][Component]
pub struct EntityAnimationPlugin<A> {
    _animation: PhantomData<fn() -> A>,
}

impl<A: EntityAnimation> Default for EntityAnimationPlugin<A> {
    fn default() -> Self {
        Self {
            _animation: PhantomData,
        }
    }
}

impl<A: EntityAnimation> Plugin for EntityAnimationPlugin<A> {
    fn build(&self, app: &mut App) {
        app.add_systems(
            A::schedule(),
            (
                EntityAnimationPlugin::<A>::will_tick,
                EntityAnimationPlugin::<A>::tick,
                EntityAnimationPlugin::<A>::did_tick,
            )
                .chain(),
        )
        .add_observer(on_insert::<A>)
        .add_observer(on_remove::<A>);
    }
}

impl<A: EntityAnimation> EntityAnimationPlugin<A> {
    /// system that runs immediately before this plugin instance's tick system,
    /// exposed for ordering
    pub fn will_tick() {}

    /// system that runs immediately after this plugin instance's tick system,
    /// exposed for ordering
    pub fn did_tick() {}

    fn tick(
        mut animations: Query<(Entity, &mut A, &mut EntityAnimationState<A>)>,
        mut param: StaticSystemParam<<A as EntityAnimation>::Param>,
        mut commands: Commands,
        time: Res<Time>,
    ) {
        for (entity, mut animation, mut state) in animations.iter_mut() {
            if state.finished() {
                continue;
            }

            state.tick(time.delta_secs(), entity, &mut *animation, &mut param);

            if state.just_repeated() && state.configuration.events {
                commands
                    .entity(entity)
                    .trigger(entity_animation_repeated::<A>);
            }

            if state.finished() {
                if state.configuration.events {
                    commands
                        .entity(entity)
                        .trigger(entity_animation_finished::<A>);
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

fn on_insert<A: EntityAnimation>(
    add_animation: On<Insert, A>,
    mut commands: Commands,
    animations: Query<&A>,
) {
    // this serves as the initialization phase
    let Ok(animation) = animations.get(add_animation.entity) else {
        return;
    };
    let mut state = EntityAnimationState::<A>::default();
    state.reset(animation);
    commands.entity(add_animation.entity).insert(state);
}

fn on_remove<A: EntityAnimation>(state: On<Remove, A>, mut commands: Commands) {
    commands
        .entity(state.entity)
        .queue_silenced(|mut entity: EntityWorldMut| {
            entity.remove::<EntityAnimationState<A>>();
        });
}

/// Observe an entity to get notified when an animation finishes
#[derive(EntityEvent)]
pub struct EntityAnimationFinished<A: EntityAnimation>(
    #[event_target] Entity,
    PhantomData<fn() -> A>,
);

fn entity_animation_finished<A: EntityAnimation>(entity: Entity) -> EntityAnimationFinished<A> {
    EntityAnimationFinished(entity, PhantomData)
}

/// Observe an entity to get notified when an animation repeats
#[derive(EntityEvent)]
pub struct EntityAnimationRepeated<A: EntityAnimation>(
    #[event_target] Entity,
    PhantomData<fn() -> A>,
);

fn entity_animation_repeated<A: EntityAnimation>(entity: Entity) -> EntityAnimationRepeated<A> {
    EntityAnimationRepeated(entity, PhantomData)
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

    pub const fn finished(&self) -> bool {
        self.finished
    }

    pub const fn paused(&self) -> bool {
        self.paused
    }
}

impl<A> From<&EntityAnimationState<A>> for AnimationState {
    fn from(value: &EntityAnimationState<A>) -> Self {
        value.state
    }
}

#[derive(Component, Clone, Copy)]
struct EntityAnimationState<A> {
    // the public part of the state
    state: AnimationState,
    // the elapsed timer for delays
    delay: f32,
    // if the animation just repeated
    just_repeated: bool,
    _animation: PhantomData<fn() -> A>,
}

impl<A> Deref for EntityAnimationState<A> {
    type Target = AnimationState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<A> DerefMut for EntityAnimationState<A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

impl<A> Default for EntityAnimationState<A> {
    fn default() -> Self {
        EntityAnimationState {
            state: Default::default(),
            delay: 0.0,
            just_repeated: false,
            _animation: PhantomData,
        }
    }
}

impl<A: EntityAnimation> EntityAnimationState<A> {
    fn finished(&self) -> bool {
        self.state.finished
    }

    fn just_repeated(&mut self) -> bool {
        let just_repeated = self.just_repeated;
        self.just_repeated = false;
        just_repeated
    }

    fn reset(&mut self, animation: &A) {
        let configuration = animation.configuration().into();
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

    fn tick(
        &mut self,
        dt: f32,
        entity: Entity,
        animation: &mut A,
        param: &mut StaticSystemParam<<A as EntityAnimation>::Param>,
    ) {
        // paused and finished animations do nothing
        if self.state.paused || self.state.finished {
            return;
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
                return;
            }
        }
        let dt = dt + leftover_dt;

        // now we get to  ask configuration for what really happened!
        let Some((t, dt, finished)) = self.state.configuration.t(self.state.elapsed, dt) else {
            // if None, we're outside the ticking range, just accumulate
            self.state.last_dt = dt;
            self.state.elapsed += dt;
            return;
        };
        // otherwise accumulate what might be trimmed
        self.state.last_dt = dt;
        self.state.elapsed += dt;

        animation.tick(entity, t, dt, param);

        if finished {
            self.state.repetition_finished();
        }
    }
}

/// control all aspects of animations, immediately
#[derive(SystemParam)]
pub struct AnimationController<'w, 's, A: EntityAnimation> {
    animations: Query<'w, 's, (Write<A>, Write<EntityAnimationState<A>>)>,
}

pub type SAnimationController<A> = AnimationController<'static, 'static, A>;

impl<'w, 's, A: EntityAnimation> AnimationController<'w, 's, A> {
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

    pub fn restart(&mut self, entity: Entity) -> &mut Self {
        if let Ok((animation, mut state)) = self.animations.get_mut(entity) {
            state.reset(&animation);
        }
        self
    }

    pub fn restart_all(&mut self) -> &mut Self {
        for (animation, mut state) in self.animations.iter_mut() {
            state.reset(&animation);
        }
        self
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

impl AnimationCommands for Commands<'_, '_> {
    fn restart<A: EntityAnimation>(&mut self, entity: Entity) -> &mut Self {
        self.queue(move |world: &mut World| {
            let mut query = world.query::<(&mut EntityAnimationState<A>, &A)>();
            let Ok((mut state, animation)) = query.get_mut(world, entity) else {
                return;
            };
            state.reset(animation);
        });
        self
    }

    fn restart_all<A: EntityAnimation>(&mut self) -> &mut Self {
        self.queue(|world: &mut World| {
            let mut query = world.query::<(&mut EntityAnimationState<A>, &A)>();
            for (mut state, animation) in query.iter_mut(world) {
                state.reset(animation);
            }
        });
        self
    }

    fn flip_pause<A: EntityAnimation>(&mut self, entity: Entity) -> &mut Self {
        self.queue(move |world: &mut World| {
            if let Some(mut state) = world.get_mut::<EntityAnimationState<A>>(entity) {
                state.paused = !state.paused;
            }
        });
        self
    }

    fn pause<A: EntityAnimation>(&mut self, entity: Entity) -> &mut Self {
        self.queue(move |world: &mut World| {
            if let Some(mut state) = world.get_mut::<EntityAnimationState<A>>(entity) {
                state.paused = true;
            }
        });
        self
    }

    fn unpause<A: EntityAnimation>(&mut self, entity: Entity) -> &mut Self {
        self.queue(move |world: &mut World| {
            if let Some(mut state) = world.get_mut::<EntityAnimationState<A>>(entity) {
                state.paused = false;
            }
        });
        self
    }

    fn flip_pause_all<A: EntityAnimation>(&mut self) -> &mut Self {
        self.queue(move |world: &mut World| {
            let mut query = world.query::<&mut EntityAnimationState<A>>();
            for mut state in query.iter_mut(world) {
                state.paused = !state.paused;
            }
        });
        self
    }

    fn pause_all<A: EntityAnimation>(&mut self) -> &mut Self {
        self.queue(move |world: &mut World| {
            let mut query = world.query::<&mut EntityAnimationState<A>>();
            for mut state in query.iter_mut(world) {
                state.paused = true;
            }
        });
        self
    }

    fn unpause_all<A: EntityAnimation>(&mut self) -> &mut Self {
        self.queue(move |world: &mut World| {
            let mut query = world.query::<&mut EntityAnimationState<A>>();
            for mut state in query.iter_mut(world) {
                state.paused = false;
            }
        });
        self
    }
}

#[cfg(test)]
#[allow(refining_impl_trait)]
mod test {
    use float_eq::assert_float_eq;
    use std::ops::DerefMut;

    use super::*;
    use bevy::prelude::*;
    use bevy_time::TimePlugin;

    #[derive(Component)]
    #[require(TestTarget)]
    struct TestAnimation {
        duration: f32,
    }

    #[derive(Component, Default, Debug)]
    struct TestTarget {
        local: usize,
        t: f32,
        dt: f32,
    }

    impl EntityAnimation for TestAnimation {
        type Param = (
            SSingle<Write<TestTarget>, With<TestAnimation>>,
            SLocal<usize>,
        );

        fn configuration(&self) -> f32 {
            self.duration
        }

        fn tick(
            &mut self,
            _entity: Entity,
            t: f32,
            dt: f32,
            param: &mut StaticSystemParam<Self::Param>,
        ) {
            let (target, tracker) = param.deref_mut();
            // this do-si-do is to verify i got the system param stuff in workable state
            **tracker += 1;
            target.local = **tracker;
            target.t = t;
            target.dt += dt;
        }
    }

    #[test]
    fn test_basics() {
        let mut app = App::new();
        app.add_plugins((
            TimePlugin,
            EntityAnimationPlugin::<TestAnimation>::default(),
        ))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(TestAnimation { duration: 1.0 });
        });
        // back to back to back to back frames
        app.update();
        app.update();
        app.update();
        app.update();
        let mut query = app.world_mut().query::<&TestTarget>();
        let target = query.single(app.world()).unwrap();
        // pretty basic test, just make sure we got called and time moves like we expect
        // (exact times depend on the timing of calling update so we aren't going to check that, but
        // more than zero, and dt accumulation == t checks things work as expected)
        assert_eq!(target.local, 4);
        assert!(target.t > 0.0);
        assert_float_eq!(target.t, target.dt, abs <= 0.001);
    }

    // need to verify more!
    // - multiple entities, same component
    // - multiple components, same entity
    // - check every in-trait option is respected
    // - controls/commands
    // - different schedules, system ordering

    // and while i'm making notes
    // - more direct domain control?
    // - more interesting playback control?
    // look over other animation systems, see what makes sense here
    // overloading `domain` to do so much is a little too old-school clever
    // one option is a single method that returns a configuration struct with reasonable defaults
}
