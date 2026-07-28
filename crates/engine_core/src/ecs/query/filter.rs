use std::{any::TypeId, marker::PhantomData};

use crate::ecs::components::{Component, archetype::Archetype};

/// Restricts which archetypes a query matches, without fetching any data itself.
pub trait QueryFilter {
    fn matches_archetype(archetype: &Archetype) -> bool;
}

impl QueryFilter for () {
    fn matches_archetype(_archetype: &Archetype) -> bool {
        true
    }
}

pub struct With<T>(PhantomData<T>);
impl<T: Component> QueryFilter for With<T> {
    fn matches_archetype(archetype: &Archetype) -> bool {
        archetype.columns.contains_key(&TypeId::of::<T>())
    }
}

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
