use macros::Component;

use crate::ecs::entities::Entity;

#[derive(Component, Clone, Debug)]
pub struct Parent(pub Entity);

#[derive(Component, Clone, Debug)]
pub struct Children(pub Vec<Entity>);
