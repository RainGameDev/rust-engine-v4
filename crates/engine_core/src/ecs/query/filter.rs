use std::{any::TypeId, marker::PhantomData};

use crate::ecs::components::{Component, archetype::Archetype};

/// Restricts which archetypes a query matches, without fetching any data itself.
///
/// Built-in filters: [`With<T>`], [`Without<T>`]. Combine with tuples:
/// `(With<A>, Without<B>)`.
pub trait QueryFilter {
    fn matches_archetype(archetype: &Archetype) -> bool;
}

impl QueryFilter for () {
    fn matches_archetype(_archetype: &Archetype) -> bool {
        true
    }
}

/// Filter: only match entities that **have** component `T`.
///
/// ```ignore
/// let q: Query<&Position, With<Player>> = Query::new(&world);
/// ```
pub struct With<T>(PhantomData<T>);
impl<T: Component> QueryFilter for With<T> {
    fn matches_archetype(archetype: &Archetype) -> bool {
        archetype.columns.contains_key(&TypeId::of::<T>())
    }
}

/// Filter: only match entities that **lack** component `T`.
///
/// ```ignore
/// let q: Query<Entity, Without<Parent>> = Query::new(&world);
/// ```
pub struct Without<T>(PhantomData<T>);
impl<T: Component> QueryFilter for Without<T> {
    fn matches_archetype(archetype: &Archetype) -> bool {
        !archetype.columns.contains_key(&TypeId::of::<T>())
    }
}

macro_rules! impl_query_filter_tuple {
    ($($t:ident),+) => {
        impl<$($t: QueryFilter),+> QueryFilter for ($($t,)+) {
            fn matches_archetype(archetype: &Archetype) -> bool {
                $($t::matches_archetype(archetype))&&+
            }
        }
    };
}

impl_query_filter_tuple!(A, B);
impl_query_filter_tuple!(A, B, C);
impl_query_filter_tuple!(A, B, C, D);
impl_query_filter_tuple!(A, B, C, D, E);
impl_query_filter_tuple!(A, B, C, D, E, F);
impl_query_filter_tuple!(A, B, C, D, E, F, G);
impl_query_filter_tuple!(A, B, C, D, E, F, G, H);
impl_query_filter_tuple!(A, B, C, D, E, F, G, H, I);
