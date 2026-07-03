//! Loads Brickadia's new save formats (`.brdb` Worlds, `.brz` Prefabs) via the
//! `brdb` crate and adapts them into the legacy `brickadia::save::SaveData`
//! structure the rest of the renderer already consumes.
//!
//! Only the main static grid (grid 1) is read. Dynamic brick grids live on
//! separate entity-relative grids and need entity transforms to place
//! correctly, so they are skipped for now. Components (lights, etc.) are also
//! not yet translated, so brick-driven lights will not appear.

use std::{collections::HashMap, path::Path};

use brickadia::save::{
    Brick, BrickColor, Collision, Color, Direction, Rotation, SaveData, Size,
};
use brdb::{
    BrFsReader, BrReader, Brdb, BrickType, Brz, Direction as BrdbDirection, IntoReader,
    Rotation as BrdbRotation,
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

    // The main static grid is always id 1.
    for chunk in reader.brick_chunk_index(1)? {
        let soa = reader.brick_chunk_soa(1, chunk.index)?;
        for brick in soa.iter_bricks(chunk.index, global.clone()) {
            let brick = brick?;
            save.bricks.push(convert_brick(
                &brick,
                &mut asset_names,
                &mut asset_lookup,
                &mut material_names,
                &mut material_lookup,
            ));
        }
    }

    save.header1.brick_count = save.bricks.len() as u32;
    save.header2.brick_assets = asset_names;
    save.header2.materials = material_names;
    // All colors are emitted as unique per-brick, so no palette is needed.
    save.header2.colors = Vec::new();

    Ok(save)
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
