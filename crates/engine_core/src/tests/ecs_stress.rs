use std::time::Instant;

use crate::ecs::World;
use crate::ecs::entities::Entity;
use macros::Component;

use rand::RngExt;
use rand::rngs::ThreadRng;
use rand::seq::IndexedRandom;

#[derive(Debug, Clone, Component)]
#[allow(unused)]
struct Position {
    x: f32,
    y: f32,
    z: f32,
}

#[allow(unused)]
#[derive(Debug, Clone, Component)]
struct Velocity {
    dx: f32,
    dy: f32,
    dz: f32,
}

#[derive(Debug, Clone, Component)]
#[allow(unused)]
struct Health {
    current: f32,
    max: f32,
}

#[allow(unused)]
#[derive(Debug, Clone, Component)]
struct Name {
    value: String,
}

#[derive(Debug, Clone, Component)]
#[allow(unused)]
struct Team {
    id: u8,
}

#[derive(Debug, Clone, Component)]
#[allow(unused)]
struct Damage {
    amount: f32,
}

type Inserter = fn(&mut World, Entity, &mut ThreadRng);

fn insert_position(world: &mut World, e: Entity, rng: &mut ThreadRng) {
    world.insert_component(
        e,
        Box::new(Position {
            x: rng.random_range(-1000.0..1000.0),
            y: rng.random_range(-1000.0..1000.0),
            z: rng.random_range(-1000.0..1000.0),
        }),
    );
}
fn insert_velocity(world: &mut World, e: Entity, rng: &mut ThreadRng) {
    world.insert_component(
        e,
        Box::new(Velocity {
            dx: rng.random_range(-10.0..10.0),
            dy: rng.random_range(-10.0..10.0),
            dz: rng.random_range(-10.0..10.0),
        }),
    );
}
fn insert_health(world: &mut World, e: Entity, rng: &mut ThreadRng) {
    world.insert_component(
        e,
        Box::new(Health {
            current: rng.random_range(1.0..100.0),
            max: 100.0,
        }),
    );
}
fn insert_name(world: &mut World, e: Entity, rng: &mut ThreadRng) {
    world.insert_component(
        e,
        Box::new(Name {
            value: format!("Entity{}", rng.random_range(0..1_000_000)),
        }),
    );
}
fn insert_team(world: &mut World, e: Entity, rng: &mut ThreadRng) {
    world.insert_component(
        e,
        Box::new(Team {
            id: rng.random_range(0..4),
        }),
    );
}
fn insert_damage(world: &mut World, e: Entity, rng: &mut ThreadRng) {
    world.insert_component(
        e,
        Box::new(Damage {
            amount: rng.random_range(1.0..50.0),
        }),
    );
}

const INSERTERS: &[Inserter] = &[
    insert_position,
    insert_velocity,
    insert_health,
    insert_name,
    insert_team,
    insert_damage,
];

#[test]
#[ignore]
// slow  run explicitly with `cargo test --release -- --ignored archetype_stress`
fn archetype_stress_one_million_entities() {
    let mut world = World::new();
    let mut rng = rand::rng();
    let entity_total = 5_000_000_000;

    let spawn_start = Instant::now();

    for _ in 0..entity_total {
        let entity = world.spawn();

        let chosen: Vec<&Inserter> = INSERTERS.sample(&mut rng, 3).collect();
        for inserter in chosen {
            inserter(&mut world, entity, &mut rng);
        }
    }

    let spawn_elapsed = spawn_start.elapsed();

    println!("\n=== Archetype Stress Test ===");
    println!("Entities spawned:   {}", entity_total);
    println!("Total time:         {:.3?}", spawn_elapsed);
    println!(
        "Avg per entity:     {:.3?}",
        spawn_elapsed / entity_total as u32
    );
    println!("Archetypes created: {}", world.archetypes().len());
    println!("Live entity count:  {}", world.entity_count());
    println!();

    let mut archetypes: Vec<_> = world.archetypes().iter().collect();
    archetypes.sort_by_key(|a| std::cmp::Reverse(a.len()));

    for (i, archetype) in archetypes.iter().enumerate() {
        println!("[{i}] {:?}", archetype);
    }

    let non_empty = archetypes.iter().filter(|a| a.len() > 0).count();
    println!("\nNon-empty archetypes: {non_empty} (expected up to 20)");

    let total_in_archetypes: usize = archetypes.iter().map(|a| a.len()).sum();
    assert_eq!(
        total_in_archetypes, entity_total,
        "entity count mismatch across archetypes"
    );
}
