use std::marker::PhantomData;

use anyhow::Result;

use crate::ecs::{
    World,
    query::{filter::QueryFilter, query::Query, world_query::WorldQuery},
};

/// Fetches exactly one entity matching the query. Returns an error if 0 or >1 match.
///
/// Derefs to the inner `Q::Item`, so you can access fields directly.
///
/// # Examples
///
/// ```ignore
/// // Immutable - get the one Player's position
/// let s: Single<&Position, With<Player>> = Single::new(&world).unwrap();
/// println!("{}", s.x);
///
/// // Mutable
/// let mut s: Single<&mut Position, With<Player>> = Single::new(&world).unwrap();
/// s.x = 42.0;
///
/// // Tuple query
/// let s: Single<(&mut Transform, &Player), With<MainPlayer>> = Single::new(&world).unwrap();
/// ```
///
/// As a system parameter:
/// ```ignore
/// #[update]
/// fn movement(
///     mut player: Single<(&mut Transform, &Player)>,
/// ) -> Result<()> {
///     let (transform, _) = &mut *player;
///     transform.translation.x += 1.0;
///     Ok(())
/// }
/// ```
pub struct Single<'w, Q: WorldQuery, F: QueryFilter = ()> {
    item: Q::Item<'w>,
    _marker: PhantomData<F>,
}

impl<'w, Q: WorldQuery, F: QueryFilter> Single<'w, Q, F> {
    pub fn new(world: &'w World) -> Result<Self> {
        let query = Query::<Q, F>::new(world);
        let mut iter = query.iter();
        let item = iter.next().ok_or_else(|| {
            anyhow::anyhow!(
                "Single<{}>: no matching entity found",
                std::any::type_name::<Q>()
            )
        })?;
        if iter.next().is_some() {
            anyhow::bail!(
                "Single<{}>: more than one entity matched",
                std::any::type_name::<Q>()
            );
        }
        Ok(Self {
            item,
            _marker: std::marker::PhantomData,
        })
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
