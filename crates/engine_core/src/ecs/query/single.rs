use std::marker::PhantomData;

use crate::ecs::{
    World,
    query::{filter::QueryFilter, query::Query, world_query::WorldQuery},
};

pub struct Single<'w, Q: WorldQuery, F: QueryFilter = ()> {
    item: Q::Item<'w>,
    _marker: PhantomData<F>,
}

impl<'w, Q: WorldQuery, F: QueryFilter> Single<'w, Q, F> {
    pub fn new(world: &'w World) -> Self {
        let query = Query::<Q, F>::new(world);
        let mut iter = query.iter();
        let item = iter.next().expect("Single<Q, F>: no matching entity found");
        assert!(
            iter.next().is_none(),
            "Single<Q, F>: more than one entity matched"
        );
        Self {
            item,
            _marker: PhantomData,
        }
    }
}
impl<'w, Q: WorldQuery, F: QueryFilter> std::ops::Deref for Single<'w, Q, F> {
    type Target = Q::Item<'w>;
    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

impl<'w, Q: WorldQuery, F: QueryFilter> std::ops::DerefMut for Single<'w, Q, F> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.item
    }
}
