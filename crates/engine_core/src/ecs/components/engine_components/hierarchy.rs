use macros::component;

use crate::ecs::entities::Entity;

#[component]
pub struct Parent(pub Entity);

#[component]
pub struct Children(pub Vec<Entity>);
