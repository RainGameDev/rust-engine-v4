use std::any::TypeId;

use crate::ecs::components::Component;
use crate::ecs::components::archetype::Archetype;
use crate::ecs::entities::Entity;

/// Defines what a query term fetches per row, and which archetypes qualify.
/// Implemented for `Entity`, `&T`, `&mut T`, `Option<Q>`, and tuples of these.
///
/// # Safety
///
/// Implementors must uphold the invariant that `fetch` only touches the data
/// the `matches_archetype` check guaranteed to exist, and that `&mut T` terms
/// never alias another term's data for the same entity.
pub unsafe trait WorldQuery {
    /// The value produced per matching entity.
    /// Generic over the borrow lifetime of the underlying archetype storage.
    type Item<'w>;

    /// Whether this term's data requirements are satisfied by `archetype`
    fn matches_archetype(archetype: &Archetype) -> bool;

    /// Reads/writes the value for `row` within `archetype`.
    ///
    /// # Safety
    ///
    /// The caller must ensure `matches_archetype` returned true for this
    /// archetype, `row` is in bounds, and aliasing rules for `&mut T` are upheld.
    unsafe fn fetch<'w>(archetype: &'w Archetype, row: usize) -> Self::Item<'w>;
}

unsafe impl WorldQuery for Entity {
    type Item<'w> = Entity;

    fn matches_archetype(_archetype: &Archetype) -> bool {
        true
    }

    unsafe fn fetch<'w>(archetype: &'w Archetype, row: usize) -> Self::Item<'w> {
        archetype.entities[row]
    }
}

unsafe impl<T: Component> WorldQuery for &T {
    type Item<'w> = &'w T;

    fn matches_archetype(archetype: &Archetype) -> bool {
        archetype.columns.contains_key(&TypeId::of::<T>())
    }

    unsafe fn fetch<'w>(archetype: &'w Archetype, row: usize) -> Self::Item<'w> {
        unsafe {
            let column = archetype.columns.get(&TypeId::of::<T>()).unwrap_unchecked();
            &*(column.get_raw(row) as *const T)
        }
    }
}

unsafe impl<T: Component> WorldQuery for &mut T {
    type Item<'w> = &'w mut T;

    fn matches_archetype(archetype: &Archetype) -> bool {
        archetype.columns.contains_key(&TypeId::of::<T>())
    }

    unsafe fn fetch<'w>(archetype: &'w Archetype, row: usize) -> Self::Item<'w> {
        unsafe {
            let column = archetype.columns.get(&TypeId::of::<T>()).unwrap_unchecked();
            &mut *(column.get_raw(row) as *mut T)
        }
    }
}

unsafe impl<Q: WorldQuery> WorldQuery for Option<Q> {
    type Item<'w> = Option<Q::Item<'w>>;

    // Option always "matches" at the archetype level — the archetype either
    // has the inner data (Some) or doesn't (None), both are valid.
    fn matches_archetype(_archetype: &Archetype) -> bool {
        true
    }

    unsafe fn fetch<'w>(archetype: &'w Archetype, row: usize) -> Self::Item<'w> {
        unsafe {
            if Q::matches_archetype(archetype) {
                Some(Q::fetch(archetype, row))
            } else {
                None
            }
        }
    }
}

// Tuple impls — extend up to however many terms you need in practice.
macro_rules! impl_world_query_tuple {
    ($($t:ident),+) => {
        unsafe impl<$($t: WorldQuery),+> WorldQuery for ($($t,)+) {
            type Item<'w> = ($($t::Item<'w>,)+);

            fn matches_archetype(archetype: &Archetype) -> bool {
                $($t::matches_archetype(archetype))&&+
            }

            unsafe fn fetch<'w>(archetype: &'w Archetype, row: usize) -> Self::Item<'w> {
                unsafe{
                    ($($t::fetch(archetype, row),)+)
                }
            }
        }
    };
}

impl_world_query_tuple!(A, B);
impl_world_query_tuple!(A, B, C);
impl_world_query_tuple!(A, B, C, D);
impl_world_query_tuple!(A, B, C, D, E, F);
impl_world_query_tuple!(A, B, C, D, E, F, G);
impl_world_query_tuple!(A, B, C, D, E, F, G, H);
