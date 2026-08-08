use serde::{Deserialize, Serialize};

use crate::{
    ecs::{
        World,
        components::{
            component_registry::{
                find_component_registration, find_component_registration_by_name,
            },
            engine_components::model_renderer::ModelRenderer,
        },
        entities::Entity,
        query::query::Query,
    },
    networking::Networked,
    rendering::core::model::GpuMesh,
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
            if let Some(reg) = find_component_registration(type_id)
                && let Some(ptr) = world.get_component_raw_ptr(entity, type_id)
            {
                let bytes = (reg.serialize_raw)(ptr);
                components.push((reg.type_name.to_string(), bytes));
            }
        }
        entities.push(EntitySnapshot {
            network_id: networked.id,
            components,
        });
    }

    Snapshot { entities }
}

/// Applies a snapshot received from the server to the local world.
pub fn apply_snapshot(world: &mut World, snapshot: Snapshot) {
    for entity_state in snapshot.entities {
        let entity = find_or_spawn(world, entity_state.network_id);

        for (name, bytes) in entity_state.components {
            let Some(registration) = find_component_registration_by_name(&name) else {
                continue;
            };
            let component = (registration.deserialize_raw)(&bytes);
            world.insert_component(entity, component);
        }

        if world.get_component::<ModelRenderer>(entity).is_none()
            && let Some(handle) = world.get_asset_handle::<GpuMesh>("meshes/cube.glb")
        {
            world.add_component(entity, ModelRenderer { model: handle });
        }
    }
}

fn find_or_spawn(world: &mut World, network_id: u64) -> Entity {
    let existing = {
        let query: Query<(Entity, &Networked)> = Query::new(world);
        query
            .iter()
            .find(|(_, networked)| networked.id == network_id)
            .map(|(entity, _)| entity)
    };
    match existing {
        Some(entity) => entity,
        None => {
            let entity = world.spawn();
            world.add_component(entity, Networked { id: network_id });
            entity
        }
    }
}
