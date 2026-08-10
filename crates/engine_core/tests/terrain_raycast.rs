use engine_core::ecs::World;
use engine_core::ecs::components::engine_components::transform::Transform;
use engine_core::physics::collider::ColliderShape;
use engine_core::physics::raycast::{build_collider_snapshot, raycast_colliders_raw, Ray};
use engine_core::tiles::TileMap;
use nalgebra::Vector3;

#[test]
fn terrain_raycast_hits() {
    let tile_map = TileMap::load_default().unwrap();
    let mut world = World::new();
    let entity = world.spawn();
    world.add_component(entity, Transform::default());
    world.add_component(entity, tile_map.build_collider());
    let snapshots = build_collider_snapshot(&world);
    assert_eq!(snapshots.len(), 1, "expected one collider snapshot");

    // Ray straight down from above the middle of the map.
    let ray = Ray::new(Vector3::new(10.0, 10.0, 10.0), Vector3::new(0.0, -1.0, 0.0));
    let hit = raycast_colliders_raw(&ray, 1000.0, &snapshots, None);
    assert!(hit.is_some(), "straight-down ray should hit the terrain");
    if let Some(hit) = hit {
        println!("hit at {:?}, distance {}", hit.point, hit.distance);
    }

    // Diagonal ray like a camera click.
    let ray = Ray::new(
        Vector3::new(10.0, 20.0, 10.0),
        Vector3::new(0.0, -1.0, -0.3).normalize(),
    );
    let hit = raycast_colliders_raw(&ray, 1000.0, &snapshots, None);
    assert!(hit.is_some(), "diagonal ray should hit the terrain");
    if let Some(hit) = hit {
        println!("diag hit at {:?}, distance {}", hit.point, hit.distance);
    }
}

#[test]
fn collider_shape_round_trips() {
    // Mesh variant (the heavy one) survives a bincode round trip.
    let tile_map = TileMap::load_default().unwrap();
    let collider = tile_map.build_collider();
    let ColliderShape::Mesh {
        triangles,
        bvh,
        model_path,
    } = &collider.shape
    else {
        panic!("expected a mesh collider");
    };
    let shape = ColliderShape::Mesh {
        triangles: triangles.clone(),
        bvh: bvh.clone(),
        model_path: model_path.clone(),
    };

    let bytes = bincode::serialize(&shape).unwrap();
    let decoded: ColliderShape = bincode::deserialize(&bytes).unwrap();
    assert_eq!(shape, decoded);

    // Primitive shapes too.
    let sphere = ColliderShape::Sphere { radius: 0.5 };
    let bytes = bincode::serialize(&sphere).unwrap();
    let decoded: ColliderShape = bincode::deserialize(&bytes).unwrap();
    assert_eq!(sphere, decoded);
}

/// Regression: a snapshot received from the server can carry a stale
/// `global_position` (the server doesn't always propagate it). `transform_update`
/// must refresh it before raycasts run, or player colliders sit at the origin.
#[test]
fn stale_global_position_is_refreshed_before_raycast() {
    use engine_core::ecs::components::engine_components::transform::transform_update;
    use engine_core::networking::Networked;

    let mut world = World::new();

    // Player at (5, 0.5, 5) as sent by the server: position set, global_position zero.
    let player = world.spawn();
    let mut transform = Transform::from_position(Vector3::new(5.0, 0.5, 5.0));
    transform.scale = Vector3::new(0.5, 0.5, 0.5);
    transform.global_position = Vector3::zeros();
    world.add_component(player, Networked { id: 1 });
    world.add_component(player, transform);
    world.add_component(
        player,
        engine_core::physics::collider::Collider::new_static(
            ColliderShape::Cuboid {
                size: Vector3::new(1.0, 1.0, 1.0),
            },
            Vector3::zeros(),
        ),
    );

    // Same ordering as Schedule::tick: refresh globals before update systems.
    transform_update(&mut world);

    // A ray aimed at the player's real position must hit the player, not nothing.
    let snapshots = build_collider_snapshot(&world);
    let ray = Ray::new(
        Vector3::new(5.0, 12.0, 5.0),
        Vector3::new(0.0, -1.0, 0.0),
    );
    let hit = raycast_colliders_raw(&ray, 1000.0, &snapshots, None);
    assert!(hit.is_some(), "player collider should be hittable");
    if let Some(hit) = hit {
        assert_eq!(hit.entity_id, player, "ray should hit the player entity");
    }
}
