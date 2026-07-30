use std::{
    alloc::{Layout, alloc, dealloc, handle_alloc_error, realloc},
    any::TypeId,
    collections::HashMap,
};

use crate::ecs::{
    EntityLocation,
    components::{BoxedComponent, Component, component_registry::find_component_registration},
    entities::Entity,
};

/// A group of entities that all share the exact same set of component.
pub struct Archetype {
    /// Entity IDs, index aligned with every columns storage.
    pub(crate) entities: Vec<Entity>,
    /// Dense column per component type present on this archetype.
    pub(crate) columns: HashMap<TypeId, Column>,
    /// Sorted component signature for this archetype.
    pub(crate) signature: ArchetypeSignature,
}

/// Type erased storage for a single component type
pub struct Column {
    /// Pointer to the start of the allocation
    pub(crate) data: *mut u8,
    /// Number of elements currently stored.
    pub(crate) len: usize,
    /// Number of elements the current allocation can hold before it needs to grow.
    pub(crate) capacity: usize,
    /// Size/alignment of one erased component instance.
    pub(crate) item_layout: Layout,
    /// Type erased drop , called on each element when the column is dropped or an element is removed.
    pub(crate) drop_fn: unsafe fn(*mut u8),
}

/// Stable identifier for an archetypes componentsignature.
pub type ArchetypeSignature = Vec<TypeId>;
impl Column {
    /// Creates an empty column sized for type `T`.
    pub(crate) fn new<T: 'static>() -> Self {
        Self {
            data: std::ptr::NonNull::dangling().as_ptr(),
            len: 0,
            capacity: 0,
            item_layout: Layout::new::<T>(),
            drop_fn: |ptr| unsafe { std::ptr::drop_in_place(ptr as *mut T) },
        }
    }
    /// Byte offset of element `index` within the allocation.
    #[inline]
    fn offset_of(&self, index: usize) -> isize {
        (self.item_layout.size() * index) as isize
    }
    /// Grows the backing allocation, doubling capacity.
    fn grow(&mut self) {
        if self.item_layout.size() == 0 {
            self.capacity = usize::MAX;
            return;
        }

        let new_capacity = if self.capacity == 0 {
            4
        } else {
            self.capacity * 2
        };
        let new_layout = Layout::from_size_align(
            self.item_layout.size() * new_capacity,
            self.item_layout.align(),
        )
        .expect("column layout overflow");
        let new_data = unsafe {
            if self.capacity == 0 {
                alloc(new_layout)
            } else {
                let old_layout = Layout::from_size_align(
                    self.item_layout.size() * self.capacity,
                    self.item_layout.align(),
                )
                .unwrap();
                realloc(self.data, old_layout, new_layout.size())
            }
        };
        if new_data.is_null() {
            handle_alloc_error(new_layout);
        }
        self.data = new_data;
        self.capacity = new_capacity;
    }
    /// Pushes a raw value of the column's component type onto the end.
    pub(crate) unsafe fn push_raw(&mut self, src: *const u8) {
        if self.len == self.capacity {
            self.grow();
        }
        unsafe {
            if self.item_layout.size() > 0 {
                let dst = self.data.offset(self.offset_of(self.len));
                std::ptr::copy_nonoverlapping(src, dst, self.item_layout.size());
            }
        }
        self.len += 1;
    }
    /// Removes element `index`, moving the last element into its place to stay dense.
    pub(crate) unsafe fn swap_remove(&mut self, index: usize) {
        unsafe {
            debug_assert!(index < self.len);
            let removed_ptr = self.get_raw(index);
            (self.drop_fn)(removed_ptr);
            let last = self.len - 1;
            if index != last && self.item_layout.size() > 0 {
                let last_ptr = self.data.offset(self.offset_of(last));
                std::ptr::copy_nonoverlapping(last_ptr, removed_ptr, self.item_layout.size());
            }
            self.len -= 1;
        }
    }
    /// Pointer to element `index`, for reading/writing through a typed reference at the call site.
    pub(crate) unsafe fn get_raw(&self, index: usize) -> *mut u8 {
        unsafe {
            debug_assert!(index < self.len);
            if self.item_layout.size() == 0 {
                return std::ptr::NonNull::<u8>::dangling().as_ptr();
            }
            self.data.offset(self.offset_of(index))
        }
    }
    pub(crate) unsafe fn swap_remove_forget(&mut self, index: usize) {
        unsafe {
            debug_assert!(index < self.len);
            let last = self.len - 1;
            if index != last && self.item_layout.size() > 0 {
                let last_ptr = self.data.offset(self.offset_of(last));
                let removed_ptr = self.data.offset(self.offset_of(index));
                std::ptr::copy_nonoverlapping(last_ptr, removed_ptr, self.item_layout.size());
            }
            self.len -= 1;
        }
    }
}
impl Drop for Column {
    fn drop(&mut self) {
        // Drop every live element, then free the allocation.
        for i in 0..self.len {
            unsafe { (self.drop_fn)(self.get_raw(i)) };
        }
        if self.capacity > 0 && self.capacity != usize::MAX && self.item_layout.size() > 0 {
            let layout = Layout::from_size_align(
                self.item_layout.size() * self.capacity,
                self.item_layout.align(),
            )
            .unwrap();
            unsafe { dealloc(self.data, layout) };
        }
    }
}

#[allow(unused)]
impl Archetype {
    /// Creates an empty archetype for the given signature.
    pub(crate) fn new(signature: ArchetypeSignature) -> Self {
        Self {
            entities: Vec::new(),
            columns: HashMap::new(),
            signature,
        }
    }

    /// Registers a column for component type `T` on this archetype.
    /// Must be called once per type in `signature` before any rows are pushed.
    pub(crate) fn add_column<T: 'static>(&mut self, type_id: TypeId) {
        self.columns.insert(type_id, Column::new::<T>());
    }

    /// Appends a new row (entity with no component data yet — caller pushes into
    /// each column separately). Returns the row index.
    pub(crate) fn allocate_row(&mut self, entity: Entity) -> usize {
        self.entities.push(entity);
        self.entities.len() - 1
    }

    /// Removes row `row`, swap-removing it to stay dense.
    pub(crate) fn swap_remove_row(
        &mut self,
        row: usize,
        locations: &mut HashMap<Entity, EntityLocation>,
    ) {
        let last = self.entities.len() - 1;

        for column in self.columns.values_mut() {
            unsafe { column.swap_remove(row) };
        }

        self.entities.swap_remove(row);

        if row != last {
            // The entity that used to be at `last` is now at `row`.
            let moved_entity = self.entities[row];
            if let Some(loc) = locations.get_mut(&moved_entity) {
                loc.row = row;
            }
        }
    }

    /// Number of entities currently stored in this archetype.
    pub(crate) fn len(&self) -> usize {
        self.entities.len()
    }

    pub(crate) fn write_component(&mut self, row: usize, component: BoxedComponent) {
        let type_id = component.as_ref().type_id();
        let column = self
            .columns
            .get_mut(&type_id)
            .expect("write_component called for a type this archetype doesn't store");

        let raw: *mut dyn Component = Box::into_raw(component);
        let src = raw as *mut u8;

        unsafe {
            let dst = column.get_raw(row);
            (column.drop_fn)(dst);
            if column.item_layout.size() > 0 {
                std::ptr::copy_nonoverlapping(src, dst, column.item_layout.size());
                dealloc(src, column.item_layout);
            }
        }
    }
    /// Moves the row at `old_row` in into `archetypes[new_id]`,
    /// copying data for every column the two archetypes share
    pub(crate) fn move_row(
        archetypes: &mut [Archetype],
        old_id: usize,
        new_id: usize,
        old_row: usize,
        inserted: Option<BoxedComponent>,
        locations: &mut std::collections::HashMap<Entity, EntityLocation>,
    ) -> usize {
        assert_ne!(old_id, new_id);

        // borrow both archetypes mutably
        let (old_archetype, new_archetype) = if old_id < new_id {
            let (left, right) = archetypes.split_at_mut(new_id);
            (&mut left[old_id], &mut right[0])
        } else {
            let (left, right) = archetypes.split_at_mut(old_id);
            (&mut right[0], &mut left[new_id])
        };

        let entity = old_archetype.entities[old_row];

        // Copy bytes for every column present in both archetypes.
        for (type_id, new_column) in new_archetype.columns.iter_mut() {
            if let Some(old_column) = old_archetype.columns.get_mut(type_id) {
                unsafe {
                    let src = old_column.get_raw(old_row);
                    new_column.push_raw(src);
                }
            }
        }

        // If this move is an insert, write the new component into its new column.
        if let Some(component) = inserted {
            let type_id = component.as_ref().type_id();
            let raw: *mut dyn Component = Box::into_raw(component);
            let src = raw as *mut u8;
            let column = new_archetype
                .columns
                .get_mut(&type_id)
                .expect("target archetype missing column for inserted component");
            unsafe {
                column.push_raw(src);
                if column.item_layout.size() > 0 {
                    dealloc(src, column.item_layout);
                }
            }
        }
        new_archetype.entities.push(entity);
        let new_row = new_archetype.entities.len() - 1;

        // Clean up the old row
        let last = old_archetype.entities.len() - 1;
        for (type_id, old_column) in old_archetype.columns.iter_mut() {
            unsafe {
                if new_archetype.columns.contains_key(type_id) {
                    old_column.swap_remove_forget(old_row);
                } else {
                    old_column.swap_remove(old_row);
                }
            }
        }
        old_archetype.entities.swap_remove(old_row);

        // Patch the location of whichever entity got swapped into the vacated old row.
        if old_row != last {
            let moved_entity = old_archetype.entities[old_row];
            if let Some(loc) = locations.get_mut(&moved_entity) {
                loc.row = old_row;
            }
        }

        new_row
    }
}

impl std::fmt::Debug for Archetype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let type_names: Vec<&str> = self
            .signature
            .iter()
            .map(|type_id| {
                find_component_registration(*type_id)
                    .map(|reg| reg.type_name)
                    .unwrap_or("<unregistered>")
            })
            .collect();

        f.debug_struct("Archetype")
            .field("components", &type_names)
            .field("entity_count", &self.entities.len())
            .finish()
    }
}
