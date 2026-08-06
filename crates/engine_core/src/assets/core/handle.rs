use std::{
    fmt::{Debug, Formatter, Result},
    hash::{Hash, Hasher},
    marker::PhantomData,
};

use crate::assets::Asset;

/// A handle for any type of asset
pub struct Handle<T: Asset> {
    /// The index of the asset.
    pub(crate) index: u32,
    /// The generation of the asset.
    pub(crate) generation: u32,
    /// Allows for the generic.
    pub(crate) _marker: PhantomData<fn() -> T>,
}

impl<T: Asset> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: Asset> Copy for Handle<T> {}

impl<T: Asset> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.generation == other.generation
    }
}
impl<T: Asset> Eq for Handle<T> {}

impl<T: Asset> Hash for Handle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.generation.hash(state);
    }
}

impl<T: Asset> Debug for Handle<T> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(
            f,
            "Handle<{}>({}, gen {})",
            std::any::type_name::<T>(),
            self.index,
            self.generation
        )
    }
}

impl<T: Asset> Handle<T> {
    /// A handle that is guaranteed not to reference any live asset.
    pub fn dangling() -> Self {
        Self {
            index: u32::MAX,
            generation: u32::MAX,
            _marker: PhantomData,
        }
    }
}
