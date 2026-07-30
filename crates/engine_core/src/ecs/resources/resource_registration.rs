pub use inventory;

use std::any::TypeId;

/// Metadata per type by `#[derive(Resource)]`,
/// collected automatically at startup via `inventory`
pub struct ResourceRegistration {
    pub type_id: fn() -> TypeId,
    pub type_name: &'static str,
}

inventory::collect!(ResourceRegistration);

/// Looks up registration info for a resource type by its `TypeId`
pub fn find_resource_registration(type_id: TypeId) -> Option<&'static ResourceRegistration> {
    inventory::iter::<ResourceRegistration>().find(|reg| (reg.type_id)() == type_id)
}

/// Looks up registration info by type name
/// this is mainly for serialisation
pub fn find_resource_registration_by_name(name: &str) -> Option<&'static ResourceRegistration> {
    inventory::iter::<ResourceRegistration>().find(|reg| reg.type_name == name)
}
