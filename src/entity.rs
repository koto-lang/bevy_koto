use crate::runtime::{
    make_channel, KotoReceiver, KotoRuntimePlugin, KotoSchedule, KotoSender, KotoUpdate,
    ScriptLoaded,
};

use bevy::prelude::*;
use koto::prelude::*;
use parking_lot::RwLock;
use std::sync::Arc;

/// Support for connecting Koto and Bevy entities
///
/// Entities with the [KotoEntity] component will be automatically despawned when the script no
/// longer refers to them.
pub struct KotoEntityPlugin;

impl Plugin for KotoEntityPlugin {
    fn build(&self, app: &mut App) {
        assert!(app.is_plugin_added::<KotoRuntimePlugin>());

        let (update_entity_sender, update_entity_receiver) = make_channel::<UpdateEntityEvent>();

        app.insert_resource(update_entity_sender)
            .insert_resource(update_entity_receiver)
            .add_systems(
                KotoSchedule,
                (
                    on_script_loaded.in_set(KotoUpdate::PreUpdate),
                    update_koto_entities.in_set(KotoUpdate::PostUpdate),
                ),
            )
            .add_systems(Update, koto_to_bevy_entity_events);
    }
}

fn on_script_loaded(
    mut entities: Query<&mut KotoEntity>,
    mut script_loaded_events: EventReader<ScriptLoaded>,
) {
    let mut clear_entities = false;
    for _ in script_loaded_events.read() {
        clear_entities = true;
    }
    if clear_entities {
        debug!("Marking entities as inactive");
        for mut koto_entity in entities.iter_mut() {
            koto_entity.is_active = false;
        }
    }
}

fn update_koto_entities(
    time: Res<Time>,
    mut query: Query<&mut KotoEntity>,
    mut commands: Commands,
) {
    let time_delta = time.delta_seconds_f64();

    for koto_entity in &query {
        // If ref_count is 1 then the Koto script is no longer referencing the entity,
        // so it can be despawned.
        if koto_entity.object.ref_count() == 1 || !koto_entity.is_active {
            commands.entity(koto_entity.entity.get()).despawn();
        }
    }

    query.par_iter_mut().for_each(|mut koto_entity| {
        if koto_entity.is_active && koto_entity.object.ref_count() > 1 {
            let instance = koto_entity.object.clone();
            if let Some((on_update, vm)) = koto_entity.on_update.as_mut() {
                if let Err(error) =
                    vm.call_instance_function(instance.into(), on_update.clone(), time_delta)
                {
                    error!("Error while calling Entity::on_update():\n{error}");
                }
            }
        }
    });
}

fn koto_to_bevy_entity_events(
    channel: Res<UpdateEntityReceiver>,
    mut query: Query<&mut KotoEntity>,
    mut commands: Commands,
) {
    while let Some(event) = channel.receive() {
        let bevy_entity = event.entity.get();
        let mut koto_entity = query.get_mut(bevy_entity).unwrap();
        match event.event {
            UpdateEntity::SetOnUpdate(on_update) => koto_entity.on_update = on_update,
            UpdateEntity::Despawn => commands.entity(bevy_entity).despawn(),
        }
    }
}

#[derive(Debug, Clone, Component)]
pub struct KotoEntity {
    /// The Koto object that corresponds to the Bevy entity
    pub object: KObject,
    /// The Koto->Bevy entity mapping
    pub entity: KotoEntityMapping,
    /// The Koto value that should be called on each update
    pub on_update: Option<(KValue, KotoVm)>,
    /// True if the entity should be displayed, false when transitioning away from a script
    pub is_active: bool,
}

impl KotoEntity {
    pub fn new(object: KObject, entity: KotoEntityMapping) -> Self {
        Self {
            object,
            entity,
            on_update: None,
            is_active: true,
        }
    }
}

#[derive(Clone, Event)]
pub enum UpdateEntity {
    /// Sets the function that should be called when updating the entity
    SetOnUpdate(Option<(KValue, KotoVm)>),
    /// The entity has been manually despawned from Koto
    Despawn,
}

pub type UpdateEntityEvent = KotoEntityEvent<UpdateEntity>;
pub type UpdateEntitySender = KotoSender<UpdateEntityEvent>;
type UpdateEntityReceiver = KotoReceiver<UpdateEntityEvent>;

/// A Bevy entity that can be referred to from Koto scripts
///
/// When an entity is first created in a Koto script, it needs to be referred to immediately during
/// the Koto function call, without waiting for the entity to be spawned as a Bevy entity.
///
/// Once the entity has been spawned, [KotoEntity::assign_bevy_entity] must be called to ensure that
/// future operations work correctly.
#[derive(Clone, Debug)]
pub struct KotoEntityMapping {
    bevy_entity: Arc<RwLock<Entity>>,
}

impl KotoEntityMapping {
    pub fn new() -> Self {
        Self {
            bevy_entity: Arc::new(RwLock::new(Entity::PLACEHOLDER)),
        }
    }

    /// Assigns the given Bevy entity to the Koto entity
    pub fn assign_bevy_entity(&mut self, entity: Entity) {
        let mut inner = self.bevy_entity.write();
        debug_assert!(*inner == Entity::PLACEHOLDER);
        *inner = entity;
    }

    pub fn get(&self) -> Entity {
        *self.bevy_entity.read()
    }
}

#[derive(Clone)]
pub struct KotoEntityEvent<T> {
    pub entity: KotoEntityMapping,
    pub event: T,
}

impl<T> KotoEntityEvent<T> {
    pub fn new(id: KotoEntityMapping, value: T) -> Self {
        Self {
            entity: id,
            event: value,
        }
    }
}
