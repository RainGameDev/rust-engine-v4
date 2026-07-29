pub mod components;
pub mod entities;
pub mod query;
pub mod resources;
pub mod systems;

use std::any::TypeId;
use std::collections::HashMap;

use anyhow::Result;

use crate::ecs::components::archetype::{Archetype, ArchetypeSignature};
use crate::ecs::components::component_registry::find_registration;
use crate::ecs::components::{BoxedComponent, Component};
use crate::ecs::entities::Entity;
use crate::ecs::resources::{Resource, ResourceMap};

/// Where a given entity currently lives: which archetype, and which row within it.
#[derive(Clone, Copy)]
pub(crate) struct EntityLocation {
    archetype_id: usize,
    row: usize,
}

pub struct World {
    /// Slot reuse allocator. `None` = free slot, generation tracks reuse.
    entity_slots: Vec<Option<u32>>,
    free_indices: Vec<u32>,

    /// Where each entity currently lives (archetype + row). Indexed by `entity.index`.
    locations: HashMap<Entity, EntityLocation>,

    /// All archetypes that currently exist.
    archetypes: Vec<Archetype>,
    /// Lookup from a component signature to its archetypes index.
    archetype_index: HashMap<ArchetypeSignature, usize>,

    /// All the resources in the world.
    resource_map: ResourceMap,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    pub fn new() -> Self {
        let mut world = Self {
            entity_slots: Vec::new(),
            free_indices: Vec::new(),
            locations: HashMap::new(),
            archetypes: Vec::new(),
            archetype_index: HashMap::new(),
            resource_map: ResourceMap::default(),
        };
        // Archetype 0 is always the empty archetype.
        world.archetypes.push(Archetype::new(Vec::new()));
        world.archetype_index.insert(Vec::new(), 0);
        world
    }

    // --- Entities ---

    /// Spawns a new entity with no components.
    pub fn spawn(&mut self) -> Entity {
        let entity = self.allocate_entity();
        let row = self.archetypes[0].allocate_row(entity);
        self.locations.insert(
            entity,
            EntityLocation {
                archetype_id: 0,
                row,
            },
        );
        entity
    }

    /// Despawns an entity, removing it from its archetype.
    /// Frees its slots index for reuse under a new generation.
    pub fn despawn(&mut self, entity: Entity) -> bool {
        let Some(loc) = self.locations.remove(&entity) else {
            return false;
        };
        self.archetypes[loc.archetype_id].swap_remove_row(loc.row, &mut self.locations);

        self.entity_slots[entity.index as usize] = None;
        self.free_indices.push(entity.index);
        true
    }

    // --- Components ---

    /// Adds component of type `T` to the provided entity.
    pub fn add_component<T: Component>(&mut self, entity: Entity, component: T) {
        self.insert_component(entity, Box::new(component));
    }

    /// Dynamic insertion path:
    /// takes a component,
    /// moves the entity to the archetype for,
    /// and writes the value into that archetype's typed Column via the type-erased push.
    pub(crate) fn insert_component(&mut self, entity: Entity, component: BoxedComponent) {
        let Some(&loc) = self.locations.get(&entity) else {
            return;
        };
        let type_id = (*component).as_any().type_id();

        let old_archetype = &self.archetypes[loc.archetype_id];
        if old_archetype.columns.contains_key(&type_id) {
            // Already has this component type, overwrite in place, no archetype move.
            self.archetypes[loc.archetype_id].write_component(loc.row, component);
            return;
        }

        // Make the new signature.
        let mut new_sig = old_archetype.signature.clone();
        new_sig.push(type_id);
        new_sig.sort_unstable();

        let new_archetype_id = self.get_or_create_archetype(new_sig);
        self.move_entity(entity, loc, new_archetype_id, Some(component));
    }

    /// Removes a component type from an entity, moving it to the archetype without that type.
    pub fn remove_component(&mut self, entity: Entity, type_id: TypeId) {
        let Some(&loc) = self.locations.get(&entity) else {
            return;
        };
        let old_archetype = &self.archetypes[loc.archetype_id];
        if !old_archetype.columns.contains_key(&type_id) {
            // doesn't have it, nothing to do
            return;
        }

        let mut new_sig = old_archetype.signature.clone();
        new_sig.retain(|t| *t != type_id);

        let new_archetype_id = self.get_or_create_archetype(new_sig);
        self.move_entity(entity, loc, new_archetype_id, None);
    }

    // --- Resources ---

    /// Adds the resource of type `T` from the world if it doesn't exists.
    pub fn add_resource<T: Resource>(&mut self, resource: T) {
        self.resource_map.insert(resource);
    }

    /// Removes the resource of type `T` from the world if it exists.
    pub fn remove_resource<T: Resource>(&mut self) {
        self.resource_map.remove::<T>();
    }
    /// Returns the resource of type `T` immutably.
    pub fn get<T: Resource>(&mut self) -> Result<&T> {
        self.resource_map.get::<T>()
    }
    /// Returns the resource of type `T` mutably.
    pub fn get_mut<T: Resource>(&mut self) -> Result<&mut T> {
        self.resource_map.get_mut::<T>()
    }

    // --- internal helpers ---

    fn allocate_entity(&mut self) -> Entity {
        if let Some(index) = self.free_indices.pop() {
            let generation = self.entity_slots[index as usize].unwrap_or(0) + 1;
            self.entity_slots[index as usize] = Some(generation);
            Entity { index, generation }
        } else {
            let index = self.entity_slots.len() as u32;
            self.entity_slots.push(Some(0));
            Entity {
                index,
                generation: 0,
            }
        }
    }

    /// Finds the archetype for a signature, creating it if it doesn't exist yet.
    fn get_or_create_archetype(&mut self, signature: ArchetypeSignature) -> usize {
        if let Some(&id) = self.archetype_index.get(&signature) {
            return id;
        }

        let mut archetype = Archetype::new(signature.clone());

        for &type_id in &signature {
            let registration = find_registration(type_id).unwrap_or_else(|| {
                panic!(
                    "component with TypeId {:?} was never registered — did you forget #[derive(Component)]?",
                    type_id
                )
            });
            archetype
                .columns
                .insert(type_id, (registration.create_column)());
        }

        let id = self.archetypes.len();
        self.archetypes.push(archetype);
        self.archetype_index.insert(signature, id);
        id
    }
    /// Moves an entity's row from its current archetype to a new one.
    fn move_entity(
        &mut self,
        entity: Entity,
        old_loc: EntityLocation,
        new_archetype_id: usize,
        inserted: Option<BoxedComponent>,
    ) {
        // Move shared columns' data old to new, drop remaining old row
        let new_row = Archetype::move_row(
            &mut self.archetypes,
            old_loc.archetype_id,
            new_archetype_id,
            old_loc.row,
            inserted,
            &mut self.locations,
        );
        self.locations.insert(
            entity,
            EntityLocation {
                archetype_id: new_archetype_id,
                row: new_row,
            },
        );
    }

    pub fn entity_count(&self) -> usize {
        self.locations.len()
    }
    pub fn archetypes(&self) -> &[Archetype] {
        &self.archetypes
    }
}
