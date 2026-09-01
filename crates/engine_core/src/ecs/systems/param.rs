use crate::assets::{Asset, core::handle::Handle};
use std::cell::{Ref, RefCell, RefMut};

use anyhow::Result;

use crate::{
    ecs::{
        World,
        components::{
            Component,
            engine_components::{camera::Camera, transform::Transform},
        },
        entities::Entity,
        query::{filter::QueryFilter, query::Query, single::Single, world_query::WorldQuery},
        resources::Resource,
    },
    physics::raycast::{
        build_collider_snapshot, get_camera_ray, raycast_colliders_raw, ColliderHit,
        ColliderSnapshot, Direction, Ray,
    },
};

pub trait SystemParam<'w>: Sized {
    fn fetch(world: &'w World) -> Result<Self>;
}

impl<'w, Q: WorldQuery, F: QueryFilter> SystemParam<'w> for Query<'w, Q, F> {
    fn fetch(world: &'w World) -> Result<Self> {
        Ok(Query::new(world))
    }
}

impl<'w, Q: WorldQuery, F: QueryFilter> SystemParam<'w> for Single<'w, Q, F> {
    fn fetch(world: &'w World) -> Result<Self> {
        Single::new(world)
    }
}

pub struct Res<'w, T: Resource> {
    value: Ref<'w, T>,
}

impl<'w, T: Resource> std::ops::Deref for Res<'w, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<'w, T: Resource> SystemParam<'w> for Res<'w, T> {
    fn fetch(world: &'w World) -> Result<Self> {
        let value = world.resource_map.get::<T>()?;
        Ok(Res { value })
    }
}

pub struct ResMut<'w, T: Resource> {
    value: RefMut<'w, T>,
}

impl<'w, T: Resource> std::ops::Deref for ResMut<'w, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<'w, T: Resource> std::ops::DerefMut for ResMut<'w, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<'w, T: Resource> SystemParam<'w> for ResMut<'w, T> {
    fn fetch(world: &'w World) -> Result<Self> {
        let value = world.resource_map.get_mut::<T>()?;
        Ok(ResMut { value })
    }
}

pub struct Assets<'w, T: Asset> {
    world: &'w World,
    _marker: std::marker::PhantomData<T>,
}

impl<'w, T: Asset> Assets<'w, T> {
    pub fn get(&self, handle: Handle<T>) -> Option<&T> {
        self.world.get_asset(handle)
    }

    /// Looks up the handle for an asset registered under `path`.
    pub fn get_handle(&self, path: &str) -> Option<Handle<T>> {
        self.world.get_asset_handle::<T>(path)
    }
}

impl<'w, T: Asset> SystemParam<'w> for Assets<'w, T> {
    fn fetch(world: &'w World) -> Result<Self> {
        Ok(Assets {
            world,
            _marker: std::marker::PhantomData,
        })
    }
}

/// Raycasting against the current colliders. The collider snapshot is built lazily on
/// the first cast and cached for the rest of the system invocation.
pub struct Raycast<'w> {
    world: &'w World,
    cache: RefCell<Option<Vec<ColliderSnapshot>>>,
}

impl<'w> SystemParam<'w> for Raycast<'w> {
    fn fetch(world: &'w World) -> Result<Self> {
        Ok(Raycast {
            world,
            cache: RefCell::new(None),
        })
    }
}

impl<'w> Raycast<'w> {
    fn snapshots(&self) -> Ref<'_, Vec<ColliderSnapshot>> {
        if self.cache.borrow().is_none() {
            *self.cache.borrow_mut() = Some(build_collider_snapshot(self.world));
        }
        Ref::map(self.cache.borrow(), |c| c.as_ref().unwrap())
    }

    /// Raycast from an arbitrary transform in a given direction
    pub fn cast(
        &self,
        transform: &Transform,
        distance: f32,
        direction: Direction,
        ignore_ids: Option<Vec<Entity>>,
    ) -> Option<ColliderHit> {
        let ray = get_camera_ray(transform, direction);
        let snapshots = self.snapshots();
        raycast_colliders_raw(&ray, distance, &snapshots, ignore_ids)
    }

    /// Raycast from a raw ray and origin
    pub fn cast_ray(
        &self,
        ray: &Ray,
        max_distance: f32,
        ignore_ids: Option<Vec<Entity>>,
    ) -> Option<ColliderHit> {
        let snapshots = self.snapshots();
        raycast_colliders_raw(ray, max_distance, &snapshots, ignore_ids)
    }

    /// Borrows a component from the entity that was hit.
    pub fn entity_of<T: Component>(&self, entity: Entity) -> Option<&T> {
        self.world.get_component(entity)
    }

    /// Raycast forward from the active camera
    pub fn cast_camera(&self, range: f32) -> Option<ColliderHit> {
        let query: Query<(&Camera, &Transform, Entity)> = Query::new(self.world);
        let queries: Vec<(&Camera, &Transform, Entity)> = query.iter().collect();
        let (_, transform, entity) = queries.first()?;
        let ray = Ray::new(transform.global_position, transform.global_forward());
        let snapshots = self.snapshots();

        // Ignore the camera's own collider and the colliders of its ancestors
        // (e.g. a character controller that the camera is parented to).
        let mut ignore = vec![*entity];
        let mut current = *entity;
        if let Some(parent) = self
            .world
            .get_component::<crate::ecs::components::engine_components::hierarchy::Parent>(current)
        {
            current = parent.0;
            ignore.push(current);
        }

        raycast_colliders_raw(&ray, range, &snapshots, Some(ignore))
    }
}
