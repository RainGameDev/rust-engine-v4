pub mod archetype;
pub mod component_registry;
pub mod engine_components;

use std::any::Any;
use std::fmt::Debug;

use macros::Component;

pub type BoxedComponent = Box<dyn Component>;

pub trait Component: Any + Debug + Send + Sync {
    fn clone_box(&self) -> BoxedComponent;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

impl<T> Component for T
where
    T: Any + Debug + Clone + Send + Sync,
{
    fn clone_box(&self) -> BoxedComponent {
        Box::new(self.clone())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Clone for BoxedComponent {
    fn clone(&self) -> BoxedComponent {
        self.clone_box()
    }
}
