use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    range::Range,
};

use bevy_app::{App, Plugin, Update};
use bevy_ecs::{
    component::{Component, Mutable},
    entity::Entity,
    event::EntityEvent,
    lifecycle::{Insert, Remove},
    observer::On,
    schedule::{IntoScheduleConfigs, ScheduleLabel},
    system::{Commands, Local, Query, Res, Single, StaticSystemParam, SystemParam},
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
        Update
    }

    /// Domain on the animation timeline where this animation actively gets ticked.
    ///
    /// The animation timeline is abstractly how long a component has been inserted on
    /// an entity. Unless an instance is paused, it will accumulate dt every frame. When
    /// the time accumulated is contained within the domain, [Self::tick] will be invoked.
    ///
    /// This can be longer or shorter than contained curves. Their domains are not considered at all,
    /// only the value returned here.
    ///
    /// Domains do not need to start at 0. A higher start effectively delays when the ticking begins.
    /// in this case your `t`` value will start then. For instance, a domain of 1.0..5.0
    /// will start after 1 second with a `t` values from 1.0 to 5.0
    ///
    /// If the range is in reverse, the animation will tick backward. Elapsed time still accumulates
    /// normally. For instance, a domain of 5.0..1.0 will start after 1 second and receive `t` values
    /// counting from 5.0 down to 1.0.
    ///
    /// If the low end of the range is negative, this is treated as a delay before the tick system is
    /// invoked, but `t` will start at 0. For instance -1.0..4.0 will delay one second, then tick from
    /// 0.0 to 4.0.
    ///
    /// If the whole range is negative, the app will crash and you'll feel silly.
    ///
    /// Changes to this value are noticed when a component instance is inserted. Once an animation
    /// is on an entity the domain is locked, only inserting a new instance will cause changes.
    fn domain(&self) -> Range<f32>;

    /// How many times to run the animation. 0 is equivalent to starting paused, nothing will
    /// happen until some external force uses a control API to change this.
    ///
    /// Defaults to 1
    ///
    /// Changes to this value are noticed when a component instance is inserted. Once an animation
    /// is on an entity the repetition count is locked.
    fn repetitions(&self) -> u32 {
        1
    }

    /// Whether to wait to start the animation, or tick right away.
    ///
    /// Defaults to false.
    ///
    /// Changes to this value are noticed when a component instance is added. Once an animation
    /// is on an entity it will either start or it won't, that's just how time works.
    fn start_paused(&self) -> bool {
        false
    }

    /// Whether the plugin should remove this component once it has finished
    ///
    /// Defaults to true, change to false if you want to leave them around
    ///
    /// This method is invoked during the frame a component reaches the finished state, so it is
    /// live.
    fn remove_on_finish(&self) -> bool {
        true
    }

    /// Called every invocation of the schedule that an instance is active
    ///
    /// - `entity` - the entity holding this component
    /// - `t` - total seconds this animation has been running
    /// - `dt` - delta time from last invocation
    /// - `param` - [Self::Param]
    fn tick(&mut self, entity: Entity, t: f32, dt: f32, param: &mut StaticSystemParam<Self::Param>);

    /// bring `t` into [0, 1] over the current domain
    fn normalized_t(&self, t: f32) -> f32 {
        // negatives in the domain range are delays outside the tick domain, so we
        // clamp those to 0.0 here
        let (low, high) = (self.domain().start.max(0.0), self.domain().end.max(0.0));

        if low == high {
            0.0
        } else if low < high {
            (t - low) / (high - low)
        } else {
            (high - t) / (high - low)
        }
    }
}

/// a [Local] system param with 'static lifetime
pub type SLocal<T> = Local<'static, T>;

/// a terribly named [Single] with 'static lifetime
pub type SSingle<D, F> = Single<'static, 'static, D, F>;

/// Command interface to control animations commands-style. if you want something
/// more immediate, use AnimationController as a system parameter
pub trait AnimationControl {
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

            if state.just_repeated() {
                commands
                    .entity(entity)
                    .trigger(entity_animation_repeated::<A>);
            }

            if state.finished() {
                commands
                    .entity(entity)
                    .trigger(entity_animation_finished::<A>);
                if animation.remove_on_finish() {
                    commands
                        .entity(entity)
                        .queue_silenced(|mut entity: EntityWorldMut| {
                            entity.remove::<A>();
                        });
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
#[derive(Debug, Clone, Copy, Default)]
pub struct AnimationState {
    elapsed: f32,
    last_dt: f32,
    domain: Range<f32>,
    repetitions: u32,
    repetitions_remaining: u32,
    finished: bool,
    paused: bool,
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
        self.repetitions
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
    state: AnimationState,
    just_repeated: bool,
    _marker: PhantomData<fn() -> A>,
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
            just_repeated: false,
            _marker: PhantomData,
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
        self.state = AnimationState {
            elapsed: 0.0,
            last_dt: 0.0,
            domain: animation.domain(),
            repetitions: animation.repetitions(),
            repetitions_remaining: animation.repetitions(),
            finished: animation.repetitions() == 0,
            paused: animation.start_paused(),
        };
    }

    fn tick(
        &mut self,
        dt: f32,
        entity: Entity,
        animation: &mut A,
        param: &mut StaticSystemParam<<A as EntityAnimation>::Param>,
    ) {
        if self.state.paused || self.state.finished {
            return;
        }

        // we're going to cheat Time a little bit here
        let mut finished = false;
        // there might be a delay to account for
        let delay = (0.0 - self.state.domain.start.min(self.state.domain.end)).max(0.0);
        let start = self.state.domain.start.max(0.0);
        let end = self.state.domain.end.max(0.0);
        let backward = start - end;
        let last_elapsed = self.state.elapsed;
        // just assume state will accumulate normally
        self.state.last_dt = dt;
        self.state.elapsed += dt;

        if self.state.elapsed < delay {
            return;
        }
        // okay this is a little fun!
        // keep in mind, state always goes forward
        // but we make new Time instances so that
        // can go backward
        let (min, max) = if backward > 0.0 {
            (end, start)
        } else {
            (start, end)
        };

        if self.state.elapsed - delay < min {
            return;
        }
        if self.state.elapsed - delay >= max {
            // okay if we get here we have just ended
            // and we want to run with the last bit of dt
            finished = true;
            self.state.last_dt = max - last_elapsed;
            self.state.elapsed = max + delay;
        }
        // now we figure out what we really mean by elapsed!
        let (elapsed, dt) = if backward > 0.0 {
            let elapsed = (start + end) - self.state.elapsed;
            (elapsed, self.state.last_dt)
        } else {
            // very simple in forward!
            (self.state.elapsed, self.state.last_dt)
        };

        animation.tick(entity, elapsed - delay, dt, param);

        if finished {
            self.state.repetition_finished();
        }
    }
}

/// control all aspects of animations, immediately
#[derive(SystemParam)]
pub struct AnimationController<'w, 's, A: EntityAnimation> {
    animations: Query<'w, 's, (&'static mut A, &'static mut EntityAnimationState<A>)>,
}

pub type SAnimationController<A> =
    StaticSystemParam<'static, 'static, AnimationController<'static, 'static, A>>;

impl<'w, 's, A: EntityAnimation> AnimationController<'w, 's, A> {
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

impl AnimationControl for Commands<'_, '_> {
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
            world
                .get_mut::<EntityAnimationState<A>>(entity)
                .map(|mut state| {
                    state.paused = !state.paused;
                });
        });
        self
    }

    fn pause<A: EntityAnimation>(&mut self, entity: Entity) -> &mut Self {
        self.queue(move |world: &mut World| {
            world
                .get_mut::<EntityAnimationState<A>>(entity)
                .map(|mut state| {
                    state.paused = true;
                });
        });
        self
    }

    fn unpause<A: EntityAnimation>(&mut self, entity: Entity) -> &mut Self {
        self.queue(move |world: &mut World| {
            world
                .get_mut::<EntityAnimationState<A>>(entity)
                .map(|mut state| {
                    state.paused = false;
                });
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
mod test {
    use std::ops::DerefMut;

    use super::*;
    use bevy::prelude::*;
    use bevy_ecs::system::lifetimeless::*;
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

        fn domain(&self) -> Range<f32> {
            (0.0..self.duration).into()
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
        let target = dbg!(query.single(app.world()).unwrap());
        // pretty basic test, just make sure we got called and time moves like we expect
        // (exact times depend on the timing of calling update so we aren't going to check that, but
        // more than zero, and dt accumulation == t checks things work as expected)
        assert_eq!(target.local, 4);
        assert!(target.t > 0.0);
        assert_eq!(target.t, target.dt);
    }
}
