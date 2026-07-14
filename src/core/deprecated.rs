#![allow(deprecated)]
use std::marker::PhantomData;

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

use super::{AnimationConfiguration, AnimationController, ECSAnimationState, RemovalOptions};

#[cfg(test)]
mod test;

#[deprecated(
    since = "0.19.2",
    note = "Replaced by bevy_ecs_animations::ECSAnimation"
)]
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
    /// Define the [SystemParam] for the [tick](EntityAnimation::tick) method. The syntax is very similar
    /// to the way system function arguments are specified, but the lifetimes are explicit and
    /// must be `'static`
    ///
    /// For example:
    /// ```ignore
    /// #[derive(Component)]
    /// struct Animation;
    ///
    /// impl EntityAnimation for Animation { ... }  
    ///
    /// fn tick(
    ///     mut transforms: Query<&mut Transform, With<Animation>>,
    ///     other: Query<&Transform, With<Animation>>,
    ///     mut commands: Commands,
    ///     mut size: Local<usize>
    /// ) { ... }
    /// ```
    /// is getting the same arguments as
    /// ```ignore
    /// type Param: (
    ///     SQuery(Write(Transform), With<Self>),
    ///     SQuery(Read(Transform), Without<Self>),
    ///     SCommands,
    ///     SLocal<usize>,
    /// );
    /// ```
    ///
    /// Two things to watch out for
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

/// a [Local] system param with 'static lifetime
pub type SLocal<T> = Local<'static, T>;

/// a terribly named [Single] with 'static lifetime
pub type SSingle<D, F> = Single<'static, 'static, D, F>;

#[deprecated(
    since = "0.19.2",
    note = "Replaced by bevy_ecs_animations::ECSAnimationCommands"
)]
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

#[deprecated(
    since = "0.19.2",
    note = "Replaced by bevy_ecs_animations::ECSAnimationPlugin"
)]
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
        .add_observer(on_remove::<A>)
        .register_required_components::<A, ECSAnimationState<A>>();
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
        mut animations: Query<(Entity, &mut A, &mut ECSAnimationState<A>)>,
        mut param: StaticSystemParam<<A as EntityAnimation>::Param>,
        mut commands: Commands,
        time: Res<Time>,
    ) {
        for (entity, mut animation, mut state) in animations.iter_mut() {
            if state.finished() {
                continue;
            }

            state.deprecated_tick(time.delta_secs(), entity, &mut *animation, &mut param);

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
    mut animations: Query<(&A, &mut ECSAnimationState<A>)>,
) {
    // this serves as the initialization phase
    let Ok((animation, mut state)) = animations.get_mut(add_animation.entity) else {
        return;
    };
    state.reset(animation.configuration().into());
}

fn on_remove<A: EntityAnimation>(state: On<Remove, A>, mut commands: Commands) {
    commands
        .entity(state.entity)
        // silenced because if this was a despawn, bevy complains
        .queue_silenced(|mut entity: EntityWorldMut| {
            entity.remove::<ECSAnimationState<A>>();
        });
}

impl<A: EntityAnimation> ECSAnimationState<A> {
    fn deprecated_tick(
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

#[deprecated(
    since = "0.19.2",
    note = "Replaced by bevy_ecs_animations::ECSAnimationFinished"
)]
/// Observe an entity to get notified when an animation finishes
#[derive(EntityEvent)]
pub struct EntityAnimationFinished<A: EntityAnimation>(
    #[event_target] Entity,
    PhantomData<fn() -> A>,
);

fn entity_animation_finished<A: EntityAnimation>(entity: Entity) -> EntityAnimationFinished<A> {
    EntityAnimationFinished(entity, PhantomData)
}

#[deprecated(
    since = "0.19.2",
    note = "Replaced by bevy_ecs_animations::ECSAnimationRepeated"
)]
/// Observe an entity to get notified when an animation repeats
#[derive(EntityEvent)]
pub struct EntityAnimationRepeated<A: EntityAnimation>(
    #[event_target] Entity,
    PhantomData<fn() -> A>,
);

fn entity_animation_repeated<A: EntityAnimation>(entity: Entity) -> EntityAnimationRepeated<A> {
    EntityAnimationRepeated(entity, PhantomData)
}

pub type SAnimationController<A> = AnimationController<'static, 'static, A>;

impl AnimationCommands for Commands<'_, '_> {
    fn restart<A: EntityAnimation>(&mut self, entity: Entity) -> &mut Self {
        self.queue(move |world: &mut World| {
            let mut query = world.query::<(&mut ECSAnimationState<A>, &A)>();
            let Ok((mut state, animation)) = query.get_mut(world, entity) else {
                return;
            };
            state.reset(animation.configuration().into());
        });
        self
    }

    fn restart_all<A: EntityAnimation>(&mut self) -> &mut Self {
        self.queue(|world: &mut World| {
            let mut query = world.query::<(&mut ECSAnimationState<A>, &A)>();
            for (mut state, animation) in query.iter_mut(world) {
                state.reset(animation.configuration().into());
            }
        });
        self
    }

    fn flip_pause<A: EntityAnimation>(&mut self, entity: Entity) -> &mut Self {
        self.queue(move |world: &mut World| {
            if let Some(mut state) = world.get_mut::<ECSAnimationState<A>>(entity) {
                state.paused = !state.paused;
            }
        });
        self
    }

    fn pause<A: EntityAnimation>(&mut self, entity: Entity) -> &mut Self {
        self.queue(move |world: &mut World| {
            if let Some(mut state) = world.get_mut::<ECSAnimationState<A>>(entity) {
                state.paused = true;
            }
        });
        self
    }

    fn unpause<A: EntityAnimation>(&mut self, entity: Entity) -> &mut Self {
        self.queue(move |world: &mut World| {
            if let Some(mut state) = world.get_mut::<ECSAnimationState<A>>(entity) {
                state.paused = false;
            }
        });
        self
    }

    fn flip_pause_all<A: EntityAnimation>(&mut self) -> &mut Self {
        self.queue(move |world: &mut World| {
            let mut query = world.query::<&mut ECSAnimationState<A>>();
            for mut state in query.iter_mut(world) {
                state.paused = !state.paused;
            }
        });
        self
    }

    fn pause_all<A: EntityAnimation>(&mut self) -> &mut Self {
        self.queue(move |world: &mut World| {
            let mut query = world.query::<&mut ECSAnimationState<A>>();
            for mut state in query.iter_mut(world) {
                state.paused = true;
            }
        });
        self
    }

    fn unpause_all<A: EntityAnimation>(&mut self) -> &mut Self {
        self.queue(move |world: &mut World| {
            let mut query = world.query::<&mut ECSAnimationState<A>>();
            for mut state in query.iter_mut(world) {
                state.paused = false;
            }
        });
        self
    }
}
