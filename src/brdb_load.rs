//! Loads Brickadia's new save formats (`.brdb` Worlds, `.brz` Prefabs) via the
//! `brdb` crate and adapts them into the legacy `brickadia::save::SaveData`
//! structure the rest of the renderer already consumes.
//!
//! Both the main static grid (grid 1) and every dynamic brick grid are read.
//! Dynamic grids live on their own entity-relative grid, so each brick is
//! transformed into world space using the owning entity's location and
//! rotation. The renderer is an integer voxel grid and can only represent the
//! 24 axis-aligned orientations, so off-axis grid rotations are snapped to the
//! nearest one. Components (lights, etc.) are still not translated, so
//! brick-driven lights will not appear.

use std::{collections::HashMap, path::Path};

use brickadia::{
    save::{
        Brick, BrickColor, Collision, Color, Component, Direction, Rotation, SaveData, Size,
        UnrealType,
    },
    util::{rotation::rotate_direction, rotation::o2d, use_translation_table},
};
use brdb::{
    fs::BrFs,
    schema::{BrdbSchemaGlobalData, BrdbStruct},
    AsBrdbValue, BrFsReader, BrReader, Brdb, BrickType, Brz, Direction as BrdbDirection, Entity,
    IntoReader, Quat4f, Rotation as BrdbRotation, CHUNK_HALF,
};

type DynError = Box<dyn std::error::Error>;

/// Load a `.brdb` World file.
pub fn load_brdb_world(path: &Path) -> Result<SaveData, DynError> {
    build_save(&Brdb::open(path)?.into_reader())
}

/// Load a `.brz` Prefab file.
pub fn load_brz_prefab(path: &Path) -> Result<SaveData, DynError> {
    build_save(&Brz::open(path)?.into_reader())
}

/// Interns a name into `names`, returning its index.
fn intern(name: String, names: &mut Vec<String>, lookup: &mut HashMap<String, u32>) -> u32 {
    if let Some(&i) = lookup.get(&name) {
        return i;
    }
    let i = names.len() as u32;
    names.push(name.clone());
    lookup.insert(name, i);
    i
}

fn build_save<T: BrFsReader>(reader: &BrReader<T>) -> Result<SaveData, DynError> {
    let global = reader.global_data()?;

    let mut save = SaveData::default();
    save.bricks.clear();

    let mut asset_names = Vec::new();
    let mut asset_lookup = HashMap::new();
    let mut material_names = Vec::new();
    let mut material_lookup = HashMap::new();

    // Transform per dynamic grid, keyed by grid id (== entity persistent index).
    let transforms = read_grid_transforms(reader);

    for grid_id in grid_ids(reader) {
        // Grid 1 is the main global grid; its bricks are already in world space.
        let transform = if grid_id == 1 { None } else { transforms.get(&grid_id) };

        let Ok(chunks) = reader.brick_chunk_index(grid_id) else {
            continue;
        };
        for chunk in chunks {
            // Empty grids keep stale chunk-index entries whose chunk files no
            // longer exist, so skip chunks that hold no bricks.
            if chunk.num_bricks == 0 {
                continue;
            }
            let Ok(soa) = reader.brick_chunk_soa(grid_id, chunk.index) else {
                continue;
            };
            if transform.is_none() && grid_id != 1 {
                eprintln!("grid {grid_id} has no entity transform; placing at origin");
            }
            // Bricks within a chunk are yielded in the same order the component
            // chunk's brick indices reference, so remember where they land.
            let chunk_base = save.bricks.len();
            for brick in soa.iter_bricks(chunk.index, global.clone()) {
                let brick = brick?;
                let mut converted = convert_brick(
                    &brick,
                    &mut asset_names,
                    &mut asset_lookup,
                    &mut material_names,
                    &mut material_lookup,
                );
                if let Some(t) = transform {
                    apply_grid_transform(&mut converted, t);
                }
                save.bricks.push(converted);
            }

            attach_light_components(&mut save, reader, &global, grid_id, chunk.index, chunk_base);
        }
    }

    save.header1.brick_count = save.bricks.len() as u32;
    save.header2.brick_assets = asset_names;
    save.header2.materials = material_names;
    // All colors are emitted as unique per-brick, so no palette is needed.
    save.header2.colors = Vec::new();

    Ok(save)
}

/// Read a chunk's point/spot light components and attach them to the bricks
/// already pushed onto `save`, in both the per-brick component map and the
/// top-level `save.components` index the renderer reads from.
fn attach_light_components<T: BrFsReader>(
    save: &mut SaveData,
    reader: &BrReader<T>,
    global: &BrdbSchemaGlobalData,
    grid_id: usize,
    chunk: brdb::ChunkIndex,
    chunk_base: usize,
) {
    let Ok((soa, data)) = reader.component_chunk(grid_id, chunk) else {
        return;
    };

    let mut brick_indices = soa.component_brick_indices.iter();
    let mut structs = data.into_iter();

    for counter in &soa.component_type_counters {
        let idx = counter.type_index as usize;
        let type_name = global.component_type_names.get_index(idx).cloned().unwrap_or_default();
        let has_struct = global
            .component_data_struct_names
            .get(idx)
            .map(|s| s.as_str() != "None")
            .unwrap_or(false);

        for _ in 0..counter.num_instances {
            let brick_local = brick_indices.next().copied().unwrap_or(0) as usize;
            let s = if has_struct { structs.next() } else { None };

            let (key, props) = match (type_name.as_str(), &s) {
                ("Component_PointLight", Some(s)) => ("BCD_PointLight", point_light_props(s)),
                ("Component_SpotLight", Some(s)) => ("BCD_SpotLight", spot_light_props(s)),
                _ => continue,
            };

            let global_idx = chunk_base + brick_local;
            if global_idx >= save.bricks.len() {
                continue;
            }
            save.bricks[global_idx]
                .components
                .insert(key.to_string(), props);
            save.components
                .entry(key.to_string())
                .or_insert_with(|| Component {
                    version: 1,
                    brick_indices: Vec::new(),
                    properties: HashMap::new(),
                })
                .brick_indices
                .push(global_idx as u32);
        }
    }
}

fn point_light_props(s: &BrdbStruct) -> HashMap<String, UnrealType> {
    HashMap::from([
        ("bUseBrickColor".into(), UnrealType::Boolean(bool_of(s, "bUseBrickColor"))),
        ("Color".into(), UnrealType::Color(color_of(s, "Color"))),
        ("Radius".into(), UnrealType::Float(f32_of(s, "Radius"))),
        ("bCastShadows".into(), UnrealType::Boolean(bool_of(s, "bCastShadows"))),
        ("Brightness".into(), UnrealType::Float(f32_of(s, "Brightness"))),
    ])
}

fn spot_light_props(s: &BrdbStruct) -> HashMap<String, UnrealType> {
    let (roll, pitch, yaw) = rotator_of(s, "Rotation");
    HashMap::from([
        ("bUseBrickColor".into(), UnrealType::Boolean(bool_of(s, "bUseBrickColor"))),
        ("Color".into(), UnrealType::Color(color_of(s, "Color"))),
        ("Radius".into(), UnrealType::Float(f32_of(s, "Radius"))),
        ("bCastShadows".into(), UnrealType::Boolean(bool_of(s, "bCastShadows"))),
        ("Brightness".into(), UnrealType::Float(f32_of(s, "Brightness"))),
        ("InnerConeAngle".into(), UnrealType::Float(f32_of(s, "InnerConeAngle"))),
        ("OuterConeAngle".into(), UnrealType::Float(f32_of(s, "OuterConeAngle"))),
        ("Rotation".into(), UnrealType::Rotator(roll, pitch, yaw)),
    ])
}

fn f32_of(s: &BrdbStruct, k: &str) -> f32 {
    s.prop(k).and_then(|v| v.as_brdb_f32()).unwrap_or(0.0)
}

fn bool_of(s: &BrdbStruct, k: &str) -> bool {
    s.prop(k).and_then(|v| v.as_brdb_bool()).unwrap_or(false)
}

fn u8_of(s: &BrdbStruct, k: &str) -> u8 {
    s.prop(k).and_then(|v| v.as_brdb_u8()).unwrap_or(0)
}

fn color_of(s: &BrdbStruct, k: &str) -> Color {
    match s.prop(k).and_then(|v| v.as_struct()) {
        Ok(c) => Color {
            r: u8_of(c, "R"),
            g: u8_of(c, "G"),
            b: u8_of(c, "B"),
            a: u8_of(c, "A"),
        },
        Err(_) => Color { r: 255, g: 255, b: 255, a: 255 },
    }
}

/// Returns (roll, pitch, yaw) to match `UnrealType::Rotator`'s field order.
fn rotator_of(s: &BrdbStruct, k: &str) -> (f32, f32, f32) {
    match s.prop(k).and_then(|v| v.as_struct()) {
        Ok(r) => (f32_of(r, "Roll"), f32_of(r, "Pitch"), f32_of(r, "Yaw")),
        Err(_) => (0.0, 0.0, 0.0),
    }
}

/// The rigid transform of a dynamic brick grid, reduced to what the integer
/// voxel renderer can represent: an integer world offset plus one of the 24
/// axis-aligned brick orientations.
struct GridTransform {
    loc: (i32, i32, i32),
    /// Brickadia orientation number (0..24), snapped from the entity quaternion.
    orient: u8,
}

impl GridTransform {
    fn from_entity(e: &Entity) -> Self {
        Self {
            loc: (
                e.location.x.round() as i32,
                e.location.y.round() as i32,
                e.location.z.round() as i32,
            ),
            orient: snap_orientation(&e.rotation),
        }
    }
}

/// Read every dynamic brick grid entity and index it by persistent id, which
/// matches the grid folder id under `World/0/Bricks/Grids/`.
fn read_grid_transforms<T: BrFsReader>(reader: &BrReader<T>) -> HashMap<usize, GridTransform> {
    let mut map = HashMap::new();
    let Ok(chunks) = reader.entity_chunk_index() else {
        return map;
    };
    for chunk in chunks {
        let Ok(entities) = reader.entity_chunk(chunk) else {
            continue;
        };
        for e in entities {
            if !e.is_brick_grid() {
                continue;
            }
            if let Some(id) = e.id {
                map.insert(id, GridTransform::from_entity(&e));
            }
        }
    }
    map
}

/// Enumerate the grid ids present in the save. Falls back to just the main grid
/// when the `Grids` folder can't be listed (e.g. some prefabs).
fn grid_ids<T: BrFsReader>(reader: &BrReader<T>) -> Vec<usize> {
    match reader.get_fs().and_then(|fs| fs.cd("World/0/Bricks/Grids")) {
        Ok(BrFs::Folder(_, children)) => {
            let mut ids: Vec<usize> = children.keys().filter_map(|k| k.parse().ok()).collect();
            ids.sort_unstable();
            ids
        }
        _ => vec![1],
    }
}

/// Place a dynamic grid's brick into world space.
fn apply_grid_transform(b: &mut Brick, t: &GridTransform) {
    // Dynamic grid bricks are stored shifted by -CHUNK_HALF (see brdb's
    // `World::add_brick_grid`), so undo that to recover grid-local coords.
    // ASSUMPTION: verify visually; if grids land half a chunk off, drop this.
    let (px, py, pz) = b.position;
    let local = (px + CHUNK_HALF, py + CHUNK_HALF, pz + CHUNK_HALF);

    let rotated = use_translation_table(local, t.orient);
    b.position = (
        rotated.0 + t.loc.0,
        rotated.1 + t.loc.1,
        rotated.2 + t.loc.2,
    );

    // Compose the grid orientation onto the brick's own orientation.
    let (nd, nr) = rotate_direction((b.direction as u8, b.rotation as u8), o2d(t.orient));
    b.direction = direction_from_u8(nd);
    b.rotation = rotation_from_u8(nr);
}

/// Snap an entity rotation quaternion to the nearest of the 24 axis-aligned
/// brick orientations by matching how each rotates the coordinate basis.
fn snap_orientation(q: &Quat4f) -> u8 {
    const AXES: [(i32, i32, i32); 3] = [(1, 0, 0), (0, 1, 0), (0, 0, 1)];
    let mut best = 0u8;
    let mut best_score = f32::NEG_INFINITY;
    for o in 0..24u8 {
        let mut score = 0.0;
        for &axis in &AXES {
            let (rx, ry, rz) = rotate_by_quat(q, axis);
            let t = use_translation_table(axis, o);
            score += rx * t.0 as f32 + ry * t.1 as f32 + rz * t.2 as f32;
        }
        if score > best_score {
            best_score = score;
            best = o;
        }
    }
    best
}

/// Rotate an integer vector by a quaternion: v + 2w(u×v) + 2u×(u×v).
fn rotate_by_quat(q: &Quat4f, v: (i32, i32, i32)) -> (f32, f32, f32) {
    let (vx, vy, vz) = (v.0 as f32, v.1 as f32, v.2 as f32);
    let tx = 2.0 * (q.y * vz - q.z * vy);
    let ty = 2.0 * (q.z * vx - q.x * vz);
    let tz = 2.0 * (q.x * vy - q.y * vx);
    (
        vx + q.w * tx + (q.y * tz - q.z * ty),
        vy + q.w * ty + (q.z * tx - q.x * tz),
        vz + q.w * tz + (q.x * ty - q.y * tx),
    )
}

fn direction_from_u8(d: u8) -> Direction {
    match d {
        0 => Direction::XPositive,
        1 => Direction::XNegative,
        2 => Direction::YPositive,
        3 => Direction::YNegative,
        4 => Direction::ZPositive,
        _ => Direction::ZNegative,
    }
}

fn rotation_from_u8(r: u8) -> Rotation {
    match r {
        0 => Rotation::Deg0,
        1 => Rotation::Deg90,
        2 => Rotation::Deg180,
        _ => Rotation::Deg270,
    }
}

fn convert_brick(
    b: &brdb::Brick,
    asset_names: &mut Vec<String>,
    asset_lookup: &mut HashMap<String, u32>,
    material_names: &mut Vec<String>,
    material_lookup: &mut HashMap<String, u32>,
) -> Brick {
    let (asset_name, size) = match &b.asset {
        BrickType::Basic(name) => (name.to_string(), Size::Empty),
        BrickType::Procedural { asset, size } => (
            asset.to_string(),
            Size::Procedural(size.x as u32, size.y as u32, size.z as u32),
        ),
    };

    Brick {
        asset_name_index: intern(asset_name, asset_names, asset_lookup),
        size,
        position: (b.position.x, b.position.y, b.position.z),
        direction: convert_direction(&b.direction),
        rotation: convert_rotation(&b.rotation),
        collision: Collision {
            player: b.collision.player,
            weapon: b.collision.weapon,
            interaction: b.collision.interact,
            tool: b.collision.tool,
        },
        visibility: b.visible,
        material_index: intern(b.material.to_string(), material_names, material_lookup),
        physical_index: 0,
        material_intensity: b.material_intensity as u32,
        color: BrickColor::Unique(Color {
            r: b.color.r,
            g: b.color.g,
            b: b.color.b,
            a: 255,
        }),
        owner_index: 0,
        components: HashMap::new(),
    }
}

fn convert_direction(d: &BrdbDirection) -> Direction {
    match d {
        BrdbDirection::XPositive => Direction::XPositive,
        BrdbDirection::XNegative => Direction::XNegative,
        BrdbDirection::YPositive => Direction::YPositive,
        BrdbDirection::YNegative => Direction::YNegative,
        BrdbDirection::ZPositive | BrdbDirection::MAX => Direction::ZPositive,
        BrdbDirection::ZNegative => Direction::ZNegative,
    }
}

fn convert_rotation(r: &BrdbRotation) -> Rotation {
    match r {
        BrdbRotation::Deg0 => Rotation::Deg0,
        BrdbRotation::Deg90 => Rotation::Deg90,
        BrdbRotation::Deg180 => Rotation::Deg180,
        BrdbRotation::Deg270 => Rotation::Deg270,
    }
}
