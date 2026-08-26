use std::any::TypeId;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::ecs::World;
use crate::ecs::components::{BoxedComponent, Component};
use crate::ecs::entities::Entity;
use crate::ecs::resources::Resource;

enum Command {
    Spawn {
        entity: Entity,
    },
    AddComponent {
        entity: Entity,
        component: BoxedComponent,
    },
    SetName {
        entity: Entity,
        name: String,
    },
    RemoveComponent {
        entity: Entity,
        type_id: TypeId,
    },
    Despawn {
        entity: Entity,
    },
    AddResource {
        insert: Box<dyn FnOnce(&mut World) + Send>,
    },
    RemoveResource {
        insert: Box<dyn FnOnce(&mut World) + Send>,
    },
    SetParent {
        child: Entity,
        parent: Option<Entity>,
    },
}

pub struct Commands {
    counter: Arc<AtomicU32>,
    queue: Vec<Command>,
}

impl Commands {
    pub fn new(counter: Arc<AtomicU32>) -> Self {
        Self {
            counter,
            queue: Vec::new(),
        }
    }

    pub fn spawn(&mut self) -> Entity {
        let index = self.counter.fetch_add(1, Ordering::Relaxed);
        let entity = Entity {
            index,
            generation: 0,
        };
        self.queue.push(Command::Spawn { entity });
        entity
    }

    pub fn add_component<T: Component>(&mut self, entity: Entity, component: T) {
        self.queue.push(Command::AddComponent {
            entity,
            component: Box::new(component),
        });
    }

    pub fn remove_component<T: Component>(&mut self, entity: Entity) {
        self.queue.push(Command::RemoveComponent {
            entity,
            type_id: TypeId::of::<T>(),
        });
    }

    pub fn despawn(&mut self, entity: Entity) {
        self.queue.push(Command::Despawn { entity });
    }

    pub fn add_resource<T: Resource>(&mut self, resource: T) {
        self.queue.push(Command::AddResource {
            insert: Box::new(move |world| world.add_resource(resource)),
        });
    }

    pub fn remove_resource<T: Resource>(&mut self) {
        self.queue.push(Command::RemoveResource {
            insert: Box::new(move |world| world.remove_resource::<T>()),
        });
    }

    pub fn set_parent(&mut self, child: Entity, parent: Option<Entity>) {
        self.queue.push(Command::SetParent { child, parent });
    }

    pub fn clear_parent(&mut self, child: Entity) {
        self.set_parent(child, None);
    }

    pub fn apply(self, world: &mut World) {
        for command in self.queue {
            match command {
                Command::Spawn { entity } => world.spawn_reserved(entity),
                Command::AddComponent { entity, component } => {
                    world.insert_component(entity, component)
                }
                Command::RemoveComponent { entity, type_id } => {
                    world.remove_component_by_type_id(entity, type_id)
                }
                Command::SetName { entity, name } => world.set_name(entity, name),
                Command::Despawn { entity } => {
                    world.despawn(entity);
                }
                Command::AddResource { insert } => insert(world),
                Command::RemoveResource { insert } => insert(world),
                Command::SetParent { child, parent } => world.set_parent(child, parent),
            }
        }
    }
}
