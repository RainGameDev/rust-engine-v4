pub use inventory;

use std::any::TypeId;

use crate::ecs::components::{BoxedComponent, archetype::Column};

/// Metadata per type by `#[derive(Component)]`,
/// collected automatically at startup via `inventory`
pub struct ComponentRegistration {
    pub type_id: fn() -> TypeId,
    pub type_name: &'static str,
    pub create_column: fn() -> Column,

    // pub serialize: fn(&dyn Component) -> Vec<u8>,
    // pub deserialize: fn(&[u8]) -> BoxedComponent,
    pub serialize_raw: fn(*const u8) -> Vec<u8>,
    pub deserialize_raw: fn(&[u8]) -> BoxedComponent,
}

inventory::collect!(ComponentRegistration);

/// Looks up registration info for a component type by its `TypeId`
pub fn find_component_registration(type_id: TypeId) -> Option<&'static ComponentRegistration> {
    inventory::iter::<ComponentRegistration>().find(|reg| (reg.type_id)() == type_id)
}

/// Looks up registration info by type name
/// this is mainly for serialisation
pub fn find_component_registration_by_name(name: &str) -> Option<&'static ComponentRegistration> {
    inventory::iter::<ComponentRegistration>().find(|reg| reg.type_name == name)
}
