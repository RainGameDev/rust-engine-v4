use std::any::{Any, TypeId, type_name};
use std::collections::HashMap;
use std::fmt::Debug;

use anyhow::{Result, anyhow};

use crate::log_warn;
pub mod resource_registration;

pub type BoxedResource = Box<dyn Resource>;

/// A hashmap that contains all resources.
#[derive(Default, Clone)]
pub struct ResourceMap {
    pub(crate) map: HashMap<TypeId, Box<dyn Resource>>,
}

impl ResourceMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Insert a new resource into the map
    pub fn insert<T: Resource + 'static>(&mut self, resource: T) {
        if self.get::<T>().is_ok() {
            log_warn!("You can only have one reasource of a given type at a time");
            return;
        }

        self.map.insert(TypeId::of::<T>(), Box::new(resource));
    }

    /// Get a resource from the map
    pub fn get<T: Resource + 'static>(&self) -> Result<&T> {
        self.map
            .get(&TypeId::of::<T>())
            .and_then(|r| r.as_any().downcast_ref::<T>())
            .ok_or_else(|| anyhow!("[ERROR!] Resource {} not found", type_name::<T>()))
    }

    /// Get a resource mutably from the map
    pub fn get_mut<T: Resource + 'static>(&mut self) -> Result<&mut T> {
        self.map
            .get_mut(&TypeId::of::<T>())
            .and_then(|r| r.as_any_mut().downcast_mut::<T>())
            .ok_or_else(|| anyhow!("[ERROR!] Resource {} not found", type_name::<T>()))
    }

    /// Remove a resource from the map
    pub fn remove<T: Resource + 'static>(&mut self) {
        self.map.remove(&TypeId::of::<T>());
    }
}

pub trait Resource: Any + Debug + Send + Sync {
    fn clone_box(&self) -> BoxedResource;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

impl<T> Resource for T
where
    T: Any + Debug + Clone + Send + Sync,
{
    fn clone_box(&self) -> BoxedResource {
        Box::new(self.clone())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Clone for BoxedResource {
    fn clone(&self) -> BoxedResource {
        self.clone_box()
    }
}
