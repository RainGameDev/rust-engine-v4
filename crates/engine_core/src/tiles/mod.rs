use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use nalgebra::Vector3;

use crate::physics::{
    bvh::Bvh,
    collider::{Collider, ColliderShape},
};
use crate::rendering::core::model::RawMesh;
use crate::rendering::core::vertex::Vertex;

/// Edge length of a single tile in world units.
pub const TILE_SIZE: f32 = 1.0;

/// `f32` heap entry.
#[derive(Debug, PartialEq)]
struct QueueNode {
    f_score: f32,
    index: usize,
}

impl Eq for QueueNode {}

impl PartialOrd for QueueNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueueNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.f_score
            .total_cmp(&other.f_score)
            .then(self.index.cmp(&other.index))
    }
}

/// 8-directional tile adjacency with movement costs
const NEIGHBORS: [(i32, i32, f32); 8] = [
    (1, 0, 1.0),
    (-1, 0, 1.0),
    (0, 1, 1.0),
    (0, -1, 1.0),
    (1, 1, std::f32::consts::SQRT_2),
    (1, -1, std::f32::consts::SQRT_2),
    (-1, 1, std::f32::consts::SQRT_2),
    (-1, -1, std::f32::consts::SQRT_2),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileKind {
    /// Walkable ground.
    Ground,
    /// Unwalkable obstacle
    Wall,
}

#[derive(Debug, Clone, Copy)]
pub struct Tile {
    pub kind: TileKind,
    /// Height of the tile's top surface. Flat terrain uses 0.
    pub height: f32,
}

impl Tile {
    pub fn walkable(&self) -> bool {
        matches!(self.kind, TileKind::Ground)
    }
}

/// A rectangular grid of 1x1 tiles.
#[derive(Debug, Clone)]
pub struct TileMap {
    pub width: usize,
    pub depth: usize,
    pub tiles: Vec<Tile>,
}

impl TileMap {
    /// Parses an ASCII map. Each character is one tile, one row per line.
    /// `'.'` is walkable ground, `'#'` is a wall. Blank lines are ignored.
    pub fn from_ascii(text: &str) -> Result<Self> {
        let rows: Vec<&str> = text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect();
        let depth = rows.len();
        if depth == 0 {
            return Err(anyhow!("map is empty"));
        }
        let width = rows[0].len();
        if width == 0 {
            return Err(anyhow!("map rows must not be empty"));
        }

        let mut tiles = Vec::with_capacity(width * depth);
        for (z, row) in rows.iter().enumerate() {
            if row.len() != width {
                return Err(anyhow!(
                    "map row {z} has width {} but expected {width}",
                    row.len()
                ));
            }
            for ch in row.chars() {
                let kind = match ch {
                    '.' => TileKind::Ground,
                    '#' => TileKind::Wall,
                    other => return Err(anyhow!("unknown tile character '{other}'")),
                };
                tiles.push(Tile { kind, height: 0.0 });
            }
        }

        Ok(Self {
            width,
            depth,
            tiles,
        })
    }

    /// Loads a map from an ASCII file on disk.
    pub fn load(path: &Path) -> Result<Self> {
        Self::from_ascii(&std::fs::read_to_string(path)?)
    }

    /// Loads the engine's default map (`engine_core/res/maps/default.map`).
    pub fn load_default() -> Result<Self> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("res/maps/default.map");
        Self::load(&path)
    }

    /// Linear index for a tile, if in bounds.
    pub fn index(&self, x: i32, z: i32) -> Option<usize> {
        if x < 0 || z < 0 || x >= self.width as i32 || z >= self.depth as i32 {
            return None;
        }
        Some(z as usize * self.width + x as usize)
    }

    pub fn in_bounds(&self, x: i32, z: i32) -> bool {
        self.index(x, z).is_some()
    }

    pub fn tile(&self, x: i32, z: i32) -> Option<&Tile> {
        self.index(x, z).map(|idx| &self.tiles[idx])
    }

    pub fn is_walkable(&self, x: i32, z: i32) -> bool {
        self.tile(x, z).map(Tile::walkable).unwrap_or(false)
    }

    /// True if the world position sits inside a walkable tile.
    pub fn is_walkable_world(&self, pos: Vector3<f32>) -> bool {
        let (x, z) = self.tile_coord(pos);
        self.is_walkable(x, z)
    }

    /// The tile a world position falls inside.
    pub fn tile_coord(&self, pos: Vector3<f32>) -> (i32, i32) {
        (pos.x.floor() as i32, pos.z.floor() as i32)
    }

    /// Center of a tile in world space.
    pub fn tile_center(&self, x: i32, z: i32) -> Option<Vector3<f32>> {
        let idx = self.index(x, z)?;
        let height = self.tiles[idx].height;
        Some(Vector3::new(x as f32 + 0.5, height, z as f32 + 0.5))
    }

    /// First walkable tile in row major order.
    pub fn first_walkable(&self) -> Option<(i32, i32)> {
        for z in 0..self.depth as i32 {
            for x in 0..self.width as i32 {
                if self.is_walkable(x, z) {
                    return Some((x, z));
                }
            }
        }
        None
    }

    /// Closest walkable tile to `(x, z)`
    pub fn nearest_walkable(&self, x: i32, z: i32) -> Option<(i32, i32)> {
        let mut best: Option<(i32, i32)> = None;
        for zz in 0..self.depth as i32 {
            for xx in 0..self.width as i32 {
                if !self.is_walkable(xx, zz) {
                    continue;
                }
                let distance = (xx - x).abs().max((zz - z).abs());
                let better = match best {
                    Some((bx, bz)) => distance < (bx - x).abs().max((bz - z).abs()),
                    None => true,
                };
                if better {
                    best = Some((xx, zz));
                }
            }
        }
        best
    }

    /// A* path between two walkable tiles. Returns the tile path including
    /// both endpoints, or `None` if the target is unreachable/unwalkable.
    /// Diagonal steps are allowed but corner cutting is prevented.
    pub fn pathfind(&self, from: (i32, i32), to: (i32, i32)) -> Option<Vec<(i32, i32)>> {
        if !self.in_bounds(from.0, from.1) || !self.is_walkable(to.0, to.1) {
            return None;
        }
        if from == to {
            return Some(vec![from]);
        }

        let (fx, fz) = from;
        let (tx, tz) = to;
        let size = self.width * self.depth;
        let key = |x: i32, z: i32| (z as usize) * self.width + (x as usize);

        let heuristic = |x: i32, z: i32| {
            let dx = (x - tx).abs() as f32;
            let dz = (z - tz).abs() as f32;
            // Octile distance, admissible for 8-directional movement.
            (dx + dz) + (std::f32::consts::SQRT_2 - 2.0) * dx.min(dz)
        };

        let mut g_score = vec![f32::INFINITY; size];
        let mut came_from = vec![(0, 0); size];
        let mut closed = vec![false; size];
        let mut open = BinaryHeap::new();

        g_score[key(fx, fz)] = 0.0;
        open.push(Reverse(QueueNode {
            f_score: heuristic(fx, fz),
            index: key(fx, fz),
        }));

        while let Some(Reverse(QueueNode {
            f_score: _,
            index: current,
        })) = open.pop()
        {
            if closed[current] {
                continue;
            }
            closed[current] = true;

            let x = (current % self.width) as i32;
            let z = (current / self.width) as i32;
            if (x, z) == to {
                let mut path = vec![(x, z)];
                let mut cursor = (x, z);
                while cursor != from {
                    cursor = came_from[key(cursor.0, cursor.1)];
                    path.push(cursor);
                }
                path.reverse();
                return Some(path);
            }

            for (dx, dz, cost) in NEIGHBORS {
                let (nx, nz) = (x + dx, z + dz);
                if !self.is_walkable(nx, nz) {
                    continue;
                }
                // Don't cut corners between two walls/edges.
                if dx != 0
                    && dz != 0
                    && (!self.is_walkable(x + dx, z) || !self.is_walkable(x, z + dz))
                {
                    continue;
                }
                let neighbor = key(nx, nz);
                if closed[neighbor] {
                    continue;
                }
                let tentative = g_score[current] + cost;
                if tentative < g_score[neighbor] {
                    g_score[neighbor] = tentative;
                    came_from[neighbor] = (x, z);
                    open.push(Reverse(QueueNode {
                        f_score: tentative + heuristic(nx, nz),
                        index: neighbor,
                    }));
                }
            }
        }

        None
    }

    /// Builds a single merged mesh: walkable tiles as flat quads on `y = 0`,
    /// wall tiles as 1x1x1 cubes (`y` in `[0, 1]`).
    pub fn build_mesh(&self) -> RawMesh {
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        for z in 0..self.depth as i32 {
            for x in 0..self.width as i32 {
                let tx = x as f32;
                let tz = z as f32;
                match self.tiles[self.index(x, z).unwrap()].kind {
                    TileKind::Ground => push_quad(
                        &mut vertices,
                        &mut indices,
                        [
                            [tx, 0.0, tz],
                            [tx, 0.0, tz + 1.0],
                            [tx + 1.0, 0.0, tz + 1.0],
                            [tx + 1.0, 0.0, tz],
                        ],
                        [0.0, 1.0, 0.0],
                    ),
                    TileKind::Wall => push_unit_cube(&mut vertices, &mut indices, [tx, 0.0, tz]),
                }
            }
        }

        RawMesh {
            vertices,
            indices,
            material_name: "terrain".to_string(),
        }
    }

    /// Builds a static mesh collider covering the entire tile mesh.
    pub fn build_collider(&self) -> Collider {
        let mesh = self.build_mesh();
        let triangles: Vec<[Vector3<f32>; 3]> = mesh
            .indices
            .chunks_exact(3)
            .map(|idx| {
                [
                    Vector3::from(mesh.vertices[idx[0] as usize].position),
                    Vector3::from(mesh.vertices[idx[1] as usize].position),
                    Vector3::from(mesh.vertices[idx[2] as usize].position),
                ]
            })
            .collect();
        let bvh = Bvh::build(triangles.clone());
        Collider::new_static(
            ColliderShape::Mesh {
                triangles: Arc::new(triangles),
                bvh: Arc::new(bvh),
                model_path: "meshes/terrain".to_string(),
            },
            Vector3::zeros(),
        )
    }
}

/// Appends a quad to the mesh.
fn push_quad(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    corners: [[f32; 3]; 4],
    normal: [f32; 3],
) {
    let base = vertices.len() as u32;
    for corner in corners {
        vertices.push(Vertex {
            position: corner,
            normal,
            uv: [0.0, 0.0],
            joints: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        });
    }
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

/// Appends a 1x1x1 cube from `origin` (bottom corner) spanning
/// `[x, x+1] x [y, y+1] x [z, z+1]`.
fn push_unit_cube(vertices: &mut Vec<Vertex>, indices: &mut Vec<u32>, origin: [f32; 3]) {
    let [x, y, z] = origin;
    let (x1, y1, z1) = (x + 1.0, y + 1.0, z + 1.0);

    let faces: [([[f32; 3]; 4], [f32; 3]); 6] = [
        // +Y top
        (
            [[x, y1, z], [x, y1, z1], [x1, y1, z1], [x1, y1, z]],
            [0.0, 1.0, 0.0],
        ),
        // -Y bottom
        (
            [[x, y, z], [x1, y, z], [x1, y, z1], [x, y, z1]],
            [0.0, -1.0, 0.0],
        ),
        // +X
        (
            [[x1, y, z], [x1, y1, z], [x1, y1, z1], [x1, y, z1]],
            [1.0, 0.0, 0.0],
        ),
        // -X
        (
            [[x, y, z], [x, y, z1], [x, y1, z1], [x, y1, z]],
            [-1.0, 0.0, 0.0],
        ),
        // +Z
        (
            [[x, y, z1], [x1, y, z1], [x1, y1, z1], [x, y1, z1]],
            [0.0, 0.0, 1.0],
        ),
        // -Z
        (
            [[x, y, z], [x, y1, z], [x1, y1, z], [x1, y, z]],
            [0.0, 0.0, -1.0],
        ),
    ];

    for (corners, normal) in faces {
        push_quad(vertices, indices, corners, normal);
    }
}
