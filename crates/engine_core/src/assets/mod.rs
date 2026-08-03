use std::{
    any::{Any, TypeId},
    fmt::Debug,
};

use macros::Asset;

pub mod core;
pub mod models;

pub trait Asset: Send + Sync + 'static {}

#[derive(Debug, Clone, Default, Asset)]
pub struct Assets {}

/// Metadata per type by `#[derive(Asset)]`,
/// collected automatically at startup via `inventory`
pub struct AssetRegistration {
    pub type_id: fn() -> TypeId,
    pub type_name: &'static str,
    pub create_asset_map: fn() -> Box<dyn Any + Send + Sync>,
}

impl Debug for AssetRegistration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssetRegistration")
            .field("type", &self.type_name)
            .finish()
    }
}
inventory::collect!(AssetRegistration);

/// Lnooks up registration info for a asset type by its `TypeId`
pub fn find_asset_registration(type_id: TypeId) -> Option<&'static AssetRegistration> {
    inventory::iter::<AssetRegistration>().find(|reg| (reg.type_id)() == type_id)
}

/// Looks up registration info by type name
/// this is mainly for serialisation
pub fn find_asset_registration_by_name(name: &str) -> Option<&'static AssetRegistration> {
    inventory::iter::<AssetRegistration>().find(|reg| reg.type_name == name)
}
