use crate::ecs::{
    World,
    query::{filter::QueryFilter, query::Query, single::Single, world_query::WorldQuery},
};

pub trait SystemParam<'w> {
    fn fetch(world: &'w World) -> Self;
}

impl<'w, Q: WorldQuery, F: QueryFilter> SystemParam<'w> for Query<'w, Q, F> {
    fn fetch(world: &'w World) -> Self {
        Query::new(world)
    }
}

impl<'w, Q: WorldQuery, F: QueryFilter> SystemParam<'w> for Single<'w, Q, F> {
    fn fetch(world: &'w World) -> Self {
        Single::new(world)
    }
}
