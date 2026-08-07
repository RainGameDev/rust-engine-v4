use crate::ecs::World;
use crate::ecs::entities::Entity;
use crate::ecs::query::filter::{With, Without};
use crate::ecs::query::query::Query;
use crate::ecs::query::single::Single;
use macros::Component;

#[derive(Debug, Clone, Component, PartialEq)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Debug, Clone, Component, PartialEq)]
struct Velocity {
    dx: f32,
    dy: f32,
}

#[derive(Debug, Clone, Component, PartialEq)]
struct Health {
    current: f32,
}

#[derive(Debug, Clone, Component)]
struct Player;

#[derive(Debug, Clone, Component)]
struct Frozen;

macro_rules! spawn_with {
    ($world:expr, $($component:expr),+ $(,)?) => {{
        let e = $world.spawn();
        $( $world.insert_component(e, Box::new($component)); )+
        e
    }};
}

#[test]
fn query_single_component_returns_only_matching_entities() {
    let mut world = World::new();

    spawn_with!(world, Position { x: 1.0, y: 1.0 });
    spawn_with!(
        world,
        Position { x: 2.0, y: 2.0 },
        Velocity { dx: 0.0, dy: 0.0 }
    );
    spawn_with!(world, Velocity { dx: 5.0, dy: 5.0 });
    let query: Query<&Position> = Query::new(&world);
    let mut positions: Vec<Position> = query.iter().cloned().collect();
    positions.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());

    assert_eq!(positions.len(), 2);
    assert_eq!(positions[0], Position { x: 1.0, y: 1.0 });
    assert_eq!(positions[1], Position { x: 2.0, y: 2.0 });
}

#[test]
fn query_tuple_requires_all_components_present() {
    let mut world = World::new();

    spawn_with!(
        world,
        Position { x: 1.0, y: 1.0 },
        Velocity { dx: 1.0, dy: 1.0 }
    );
    spawn_with!(world, Position { x: 2.0, y: 2.0 });
    let query: Query<(&Position, &Velocity)> = Query::new(&world);
    let results: Vec<(Position, Velocity)> =
        query.iter().map(|(p, v)| (p.clone(), v.clone())).collect();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0, Position { x: 1.0, y: 1.0 });
    assert_eq!(results[0].1, Velocity { dx: 1.0, dy: 1.0 });
}

#[test]
fn query_mutable_reference_actually_mutates_storage() {
    let mut world = World::new();
    spawn_with!(
        world,
        Position { x: 0.0, y: 0.0 },
        Velocity { dx: 3.0, dy: 4.0 }
    );

    {
        let query: Query<(&mut Position, &Velocity)> = Query::new(&world);
        for (position, velocity) in query.iter() {
            position.x += velocity.dx;
            position.y += velocity.dy;
        }
    }

    let query: Query<&Position> = Query::new(&world);
    let position = query.iter().next().expect("expected one entity");
    assert_eq!(*position, Position { x: 3.0, y: 4.0 });
}

#[test]
fn query_with_filter_excludes_entities_missing_marker() {
    let mut world = World::new();

    spawn_with!(world, Position { x: 1.0, y: 1.0 }, Player);
    spawn_with!(world, Position { x: 2.0, y: 2.0 });
    let query: Query<&Position, With<Player>> = Query::new(&world);
    let results: Vec<Position> = query.iter().cloned().collect();

    assert_eq!(results, vec![Position { x: 1.0, y: 1.0 }]);
}

#[test]
fn query_without_filter_excludes_entities_with_marker() {
    let mut world = World::new();

    spawn_with!(world, Position { x: 1.0, y: 1.0 });
    spawn_with!(world, Position { x: 2.0, y: 2.0 }, Frozen);
    let query: Query<&Position, Without<Frozen>> = Query::new(&world);
    let results: Vec<Position> = query.iter().cloned().collect();

    assert_eq!(results, vec![Position { x: 1.0, y: 1.0 }]);
}

#[test]
fn query_option_returns_none_when_component_missing() {
    let mut world = World::new();

    spawn_with!(world, Position { x: 1.0, y: 1.0 }, Health { current: 50.0 });
    spawn_with!(world, Position { x: 2.0, y: 2.0 }); // no Health

    let query: Query<(&Position, Option<&Health>)> = Query::new(&world);
    let mut results: Vec<(Position, Option<Health>)> =
        query.iter().map(|(p, h)| (p.clone(), h.cloned())).collect();
    results.sort_by(|a, b| a.0.x.partial_cmp(&b.0.x).unwrap());

    assert_eq!(
        results[0],
        (Position { x: 1.0, y: 1.0 }, Some(Health { current: 50.0 }))
    );
    assert_eq!(results[1], (Position { x: 2.0, y: 2.0 }, None));
}

#[test]
fn query_entity_yields_matching_entity_ids() {
    let mut world = World::new();

    let e1 = spawn_with!(world, Player);
    spawn_with!(world, Position { x: 0.0, y: 0.0 }); // no Player

    let query: Query<Entity, With<Player>> = Query::new(&world);
    let entities: Vec<Entity> = query.iter().collect();

    assert_eq!(entities, vec![e1]);
}

#[test]
fn query_returns_empty_for_no_matches() {
    let mut world = World::new();
    spawn_with!(world, Velocity { dx: 1.0, dy: 1.0 });

    let query: Query<&Health> = Query::new(&world);
    assert_eq!(query.iter().count(), 0);
}

#[test]
fn single_returns_the_only_matching_item() {
    let mut world = World::new();
    spawn_with!(world, Position { x: 9.0, y: 9.0 }, Player);
    spawn_with!(world, Position { x: 0.0, y: 0.0 }); // not a Player, ignored

    let single: Single<&Position, With<Player>> = Single::new(&world).unwrap();
    assert_eq!(**single, Position { x: 9.0, y: 9.0 });
}

#[test]
#[should_panic(expected = "no matching entity found")]
fn single_panics_when_no_match() {
    let world = World::new();
    let _single: Single<&Position, With<Player>> = Single::new(&world).unwrap();
}

#[test]
#[should_panic(expected = "more than one entity matched")]
fn single_panics_when_multiple_matches() {
    let mut world = World::new();
    spawn_with!(world, Position { x: 1.0, y: 1.0 }, Player);
    spawn_with!(world, Position { x: 2.0, y: 2.0 }, Player);

    let _single: Single<&Position, With<Player>> = Single::new(&world).unwrap();
}

#[test]
fn single_mut_deref_allows_mutation() {
    let mut world = World::new();
    spawn_with!(world, Position { x: 0.0, y: 0.0 }, Player);

    {
        let mut single: Single<&mut Position, With<Player>> = Single::new(&world).unwrap();
        single.x = 42.0;
    }

    let query: Query<&Position, With<Player>> = Query::new(&world);
    assert_eq!(query.iter().next().unwrap().x, 42.0);
}
