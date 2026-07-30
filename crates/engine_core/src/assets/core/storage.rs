use std::{fmt::Debug, marker::PhantomData};

use crate::assets::{Asset, core::handle::Handle};

pub struct Slot<T: Asset> {
    generation: u32,
    value: Option<T>,
}

pub struct AssetMap<T: Asset> {
    slots: Vec<Slot<T>>,
    free_list: Vec<u32>,
}

impl<T: Asset> Default for AssetMap<T> {
    fn default() -> Self {
        Self {
            slots: Vec::new(),
            free_list: Vec::new(),
        }
    }
}
impl<T: Asset + Debug> Debug for AssetMap<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssetMap")
            .field("type", &std::any::type_name::<T>())
            .field("live", &self.len())
            .field("entries", &self.iter().collect::<Vec<_>>())
            .finish()
    }
}

impl<T: Asset> AssetMap<T> {
    pub fn add(&mut self, value: T) -> Handle<T> {
        if let Some(index) = self.free_list.pop() {
            let slot = &mut self.slots[index as usize];
            slot.value = Some(value);
            Handle {
                index,
                generation: slot.generation,
                _marker: PhantomData,
            }
        } else {
            let index = self.slots.len() as u32;
            self.slots.push(Slot {
                generation: 0,
                value: Some(value),
            });
            Handle {
                index,
                generation: 0,
                _marker: PhantomData,
            }
        }
    }

    /// Gets the asset of `handle` and returns it.
    pub fn get(&self, handle: Handle<T>) -> Option<&T> {
        self.slots.get(handle.index as usize).and_then(|slot| {
            (slot.generation == handle.generation)
                .then(|| slot.value.as_ref())
                .flatten()
        })
    }

    /// Gets the asset of `handle` and returns it mutably.
    pub fn get_mut(&mut self, handle: Handle<T>) -> Option<&mut T> {
        self.slots.get_mut(handle.index as usize).and_then(|slot| {
            (slot.generation == handle.generation)
                .then(|| slot.value.as_mut())
                .flatten()
        })
    }

    /// Gets the asset of `handle` removes it.
    pub fn remove(&mut self, handle: Handle<T>) -> Option<T> {
        let slot = self.slots.get_mut(handle.index as usize)?;
        if slot.generation != handle.generation {
            return None;
        }
        slot.generation = slot.generation.wrapping_add(1);
        self.free_list.push(handle.index);
        slot.value.take()
    }

    /// Does the asset of `handle` exist?
    pub fn contains(&self, handle: Handle<T>) -> bool {
        self.get(handle).is_some()
    }

    /// Returns the length of the live assets.
    pub fn len(&self) -> usize {
        self.slots.len() - self.free_list.len()
    }

    /// Are there no assets?
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Generates an iterator of all assets.
    pub fn iter(&self) -> impl Iterator<Item = (Handle<T>, &T)> {
        self.slots.iter().enumerate().filter_map(|(i, slot)| {
            slot.value.as_ref().map(|v| {
                (
                    Handle {
                        index: i as u32,
                        generation: slot.generation,
                        _marker: PhantomData,
                    },
                    v,
                )
            })
        })
    }
}
