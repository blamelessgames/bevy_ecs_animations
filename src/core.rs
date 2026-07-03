use std::marker::PhantomData;

use bevy_app::{App, Plugin, Update};
use bevy_ecs::{
    component::{Component, Mutable},
    entity::Entity,
    event::EntityEvent,
    lifecycle::{Add, Remove},
    observer::On,
    query::{QueryData, QueryFilter},
    schedule::ScheduleLabel,
    system::{Commands, Query, Res, SystemParam},
};
use bevy_time::Time;

pub struct EntityAnimationPlugin<A> {
    _marker: PhantomData<A>,
}

impl<A> Default for EntityAnimationPlugin<A> {
    fn default() -> Self {
        EntityAnimationPlugin {
            _marker: Default::default(),
        }
    }
}

impl<A: EntityAnimation> Plugin for EntityAnimationPlugin<A> {
    fn build(&self, app: &mut App) {
        app.add_systems(A::schedule(), tick::<A>)
            .add_observer(on_add::<A>)
            .add_observer(on_remove::<A>);
    }
}

fn tick<A: EntityAnimation>(
    mut animations: Query<(Entity, &mut A, &mut EntityAnimationState<A>)>,
    mut animated_components: Query<A::QueryData, A::QueryFilter>,
    mut commands: Commands,
    time: Res<Time>,
) {
    for (entity, mut animation, mut state) in animations.iter_mut() {
        if state.finished {
            continue;
        }
        let repeat = state.repeat;
        state.tick(
            &time,
            entity,
            &mut *animation,
            animated_components.reborrow(),
        );

        if repeat != state.repeat {
            commands.entity(entity).trigger(EntityAnimationRepeated);
        }

        if state.finished {
            commands.entity(entity).trigger(EntityAnimationFinished);
            if animation.remove_on_finish() {
                // the on remove handler will fix this!
                commands.entity(entity).remove::<A>();
            }
        }
    }
}

fn on_add<A: EntityAnimation>(state: On<Add, A>, mut commands: Commands, new_animation: Query<&A>) {
    let new_animation = new_animation
        .get(state.entity)
        .expect("should be unreachable in fact, this is literally the observer on add!");
    // this serves as the initialization phase and multiple animations can be queued
    // up on a single entity since the state is type-specific, which is impossible
    // to express in a require from what i can tell
    commands.entity(state.entity).insert(EntityAnimationState {
        elapsed: new_animation.skip(),
        repeat: new_animation.repeat(),
        finished: false,
        paused: new_animation.start_paused(),
        _marker: PhantomData::<A>,
    });
}

fn on_remove<A: EntityAnimation>(state: On<Remove, A>, mut commands: Commands) {
    commands
        .entity(state.entity)
        .remove::<EntityAnimationState<A>>();
}

#[derive(SystemParam)]
pub struct EntityAnimationController<'w, 's, A: EntityAnimation> {
    animations: Query<'w, 's, (&'static mut EntityAnimationState<A>, &'static mut A)>,
}

impl<'w, 's, A: EntityAnimation> EntityAnimationController<'w, 's, A> {
    pub fn is_finished(&self, entity: Entity) -> Option<bool> {
        self.animations
            .get(entity)
            .ok()
            .map(|(state, _)| state.finished)
    }

    pub fn is_paused(&self, entity: Entity) -> Option<bool> {
        self.animations
            .get(entity)
            .ok()
            .map(|(state, _)| state.paused)
    }

    pub fn change_pause(&mut self, entity: Entity, paused: bool) -> Option<bool> {
        let Ok((mut state, _)) = self.animations.get_mut(entity) else {
            return None;
        };
        let old = state.paused;
        state.paused = paused;
        Some(old)
    }
}

#[derive(EntityEvent)]
pub struct EntityAnimationFinished(Entity);

#[derive(EntityEvent)]
pub struct EntityAnimationRepeated(Entity);

#[derive(Component, Default)]
struct EntityAnimationState<A> {
    elapsed: f32,
    repeat: u32, // 4 billion repetitions ought to be enough for anyone
    finished: bool,
    paused: bool,
    _marker: PhantomData<A>,
}

impl<A: EntityAnimation> EntityAnimationState<A> {
    fn tick(
        &mut self,
        time: &Time,
        entity: Entity,
        animation: &mut A,
        mut animated_components: Query<A::QueryData, A::QueryFilter>,
    ) {
        if self.paused || self.finished {
            return;
        }

        self.elapsed = (self.elapsed + time.delta_secs()).clamp(0.0, animation.duration());
        animation.tick(
            self.elapsed,
            time.delta_secs(),
            entity,
            animated_components.reborrow(),
        );
        if self.elapsed >= animation.duration() && self.repeat > 0 {
            self.repeat -= 1;
            self.elapsed = 0.0;
        }
        self.finished = self.elapsed >= animation.duration() && self.repeat == 0;
    }
}

pub trait EntityAnimation: Component<Mutability = Mutable> {
    type QueryData: QueryData;
    type QueryFilter: QueryFilter;

    /// schedule to update the animation. probably Update is fine but change it if you need
    fn schedule() -> impl ScheduleLabel {
        Update
    }

    /// overall duration. can be longer or shorter than contained curves, it's up to you
    fn duration(&self) -> f32;

    /// times to repeat. there isn't really a "forever" but u32::MAX is unlikely to ever
    /// finish, right? and if it is just watch the event, grab the controller, and restart
    /// you're a pathological case, deal
    fn repeat(&self) -> u32 {
        0
    }

    /// time to skip in the first repetition, if you need it. not sure this is useful
    fn skip(&self) -> f32 {
        0.0
    }

    /// return true to control when you start the animation. this way you can cue up
    /// sequences without moving archetypes (since this whole thing started as "how can
    /// i animate stuff without all the overhead of being generic but without copying the
    /// danged infrastructure in every instance? and once you start saving cycles and bytes
    /// the tendency is to push it as far as you can")
    fn start_paused(&self) -> bool {
        false
    }

    /// return true if you want the aniamtion components removed when the animation finishes
    /// or don't whatever. if archetype moves bother you, have some control. if you don't
    /// care, leave it up to us
    fn remove_on_finish(&self) -> bool {
        true
    }

    /// called every invocation of the schedule with the t from 0.0->duration, the delta t
    /// the entity this animation is on, and the query. maybe soon to a system param dangit
    fn tick(
        &mut self,
        t: f32,
        dt: f32,
        entity: Entity,
        components: Query<Self::QueryData, Self::QueryFilter>,
    );
}
