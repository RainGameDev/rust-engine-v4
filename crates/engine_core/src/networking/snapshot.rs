use std::any::TypeId;

use serde::{Deserialize, Serialize};

use crate::{
    ecs::{
        World, components::component_registry::find_component_registration, entities::Entity,
        query::query::Query,
    },
    networking::Networked,
};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Snapshot {
    pub entities: Vec<EntitySnapshot>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EntitySnapshot {
    pub network_id: u64,

    pub components: Vec<(String, Vec<u8>)>,
}

pub fn build_snapshot(world: &World) -> Snapshot {
    let query: Query<(Entity, &Networked)> = Query::new(world);
    let mut entities = Vec::new();

    for (entity, networked) in query.iter() {
        let mut components = Vec::new();

        for &type_id in world.archetype_signature_of(entity) {
            if let Some(reg) = find_component_registration(type_id) {
                if let Some(ptr) = world.get_component_raw_ptr(entity, type_id) {
                    let bytes = (reg.serialize_raw)(ptr);
                    components.push((reg.type_name.to_string(), bytes));
                }
            }
        }
        entities.push(EntitySnapshot {
            network_id: networked.id,
            components,
        });
    }

    Snapshot { entities }
}
