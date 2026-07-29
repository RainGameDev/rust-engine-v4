use std::any::Any;
use std::fmt::Debug;
pub mod resource_registration;

pub type BoxedResource = Box<dyn Resource>;

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
