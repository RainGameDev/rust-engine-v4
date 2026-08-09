use crate::assets::{Asset, core::handle::Handle};
use std::cell::{Ref, RefMut};

use anyhow::Result;

use crate::ecs::{
    World,
    query::{filter::QueryFilter, query::Query, single::Single, world_query::WorldQuery},
    resources::Resource,
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
