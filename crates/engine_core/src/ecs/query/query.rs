use std::marker::PhantomData;

use crate::ecs::{
    World,
    components::archetype::Archetype,
    query::{filter::QueryFilter, world_query::WorldQuery},
};

/// Iterates over all entities matching the query terms `Q` and filter `F`.
///
/// `Q` defines what data to fetch (e.g. `&Position`, `(&Position, &Velocity)`).
/// `F` is an optional filter (defaults to no filter).
///
/// # Examples
///
/// ```ignore
/// // All entities with Position
/// let q: Query<&Position> = Query::new(&world);
///
/// // Entities with both Position and Velocity
/// let q: Query<(&Position, &Velocity)> = Query::new(&world);
///
/// // Mutable access
/// let q: Query<(&mut Position, &Velocity)> = Query::new(&world);
///
/// // Optional component (returns None when missing)
/// let q: Query<(&Position, Option<&Health>)> = Query::new(&world);
///
/// // Entity IDs
/// let q: Query<Entity> = Query::new(&world);
///
/// // With filter - only entities that have Player
/// let q: Query<&Position, With<Player>> = Query::new(&world);
///
/// // Without filter - only entities that lack Frozen
/// let q: Query<&Position, Without<Frozen>> = Query::new(&world);
///
/// // Combined filters
/// let q: Query<&Position, (With<Player>, Without<Frozen>)> = Query::new(&world);
/// ```
///
/// As a system parameter:
/// ```ignore
/// #[update]
/// fn my_system(
///     query: Query<(&mut Position, &Velocity)>,
///     filtered: Query<Entity, Without<EditorCamera>>,
/// ) -> Result<()> {
///     for (pos, vel) in &query {
///         pos.x += vel.dx;
///     }
///     Ok(())
/// }
/// ```
pub struct Query<'w, Q: WorldQuery, F: QueryFilter = ()> {
    world: &'w World,
    matching_archetypes: Vec<usize>,
    _marker: PhantomData<(Q, F)>,
}

impl<'w, Q: WorldQuery, F: QueryFilter> Query<'w, Q, F> {
    pub fn new(world: &'w World) -> Self {
        let matching_archetypes = world
            .archetypes()
            .iter()
            .enumerate()
            .filter(|(_, a)| Q::matches_archetype(a) && F::matches_archetype(a))
            .map(|(i, _)| i)
            .collect();

        Self {
            world,
            matching_archetypes,
            _marker: PhantomData,
        }
    }

    pub fn iter(&self) -> QueryIter<'w, Q> {
        QueryIter {
            world: self.world,
            archetype_ids: self.matching_archetypes.clone().into_iter(),
            current: None,
            row: 0,
            _marker: PhantomData,
        }
    }
}

pub struct QueryIter<'w, Q: WorldQuery> {
    world: &'w World,
    archetype_ids: std::vec::IntoIter<usize>,
    current: Option<&'w Archetype>,
    row: usize,
    _marker: PhantomData<Q>,
}

impl<'w, Q: WorldQuery> Iterator for QueryIter<'w, Q> {
    type Item = Q::Item<'w>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(archetype) = self.current
                && self.row < archetype.entities.len()
            {
                let item = unsafe { Q::fetch(archetype, self.row) };
                self.row += 1;
                return Some(item);
            }
            let next_id = self.archetype_ids.next()?;
            self.current = Some(&self.world.archetypes()[next_id]);
            self.row = 0;
        }
    }
}

impl<'w, Q: WorldQuery, F: QueryFilter> IntoIterator for &'_ Query<'w, Q, F> {
    type Item = Q::Item<'w>;
    type IntoIter = QueryIter<'w, Q>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
