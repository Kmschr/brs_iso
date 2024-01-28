use std::time::SystemTime;

use bevy::{prelude::*, render::render_resource::PrimitiveTopology, utils::HashMap};
use brickadia::{save::{SaveData, Size, Brick, BrickColor}, util::{BRICK_SIZE_MAP, rotation::d2o, get_axis_size}};
use lazy_static::lazy_static;

use crate::{faces::*, tri::cc};

macro_rules! rm {
    (
        r($rx:literal, $ry:literal, $rz:literal),
        u($ux:literal, $uy:literal, $uz:literal),
        f($fx:literal, $fy:literal, $fz:literal)
    ) => {
        Quat::from_mat3(&Mat3::from_cols_array(&[
            $rx, $ux, -$fx, $ry, $uy, -$fy, $rz, $uz, -$fz,
        ]))
    };
}

lazy_static! {
    static ref ORIENTATION_MAP: [Quat; 24] = [
        rm!(r(0.0, 1.0, 0.0), u(1.0, 0.0, 0.0), f(0.0, 0.0, 1.0)),    // XPositive, Deg180
        rm!(r(0.0, 1.0, 0.0), u(0.0, 0.0, -1.0), f(1.0, 0.0, 0.0)),   // YNegative, Deg180
        rm!(r(0.0, 1.0, 0.0), u(-1.0, 0.0, 0.0), f(0.0, 0.0, -1.0)),  // XNegative, Deg180
        rm!(r(0.0, 1.0, 0.0), u(0.0, 0.0, 1.0), f(-1.0, 0.0, 0.0)),   // YPositive, Deg180
        rm!(r(0.0, -1.0, 0.0), u(1.0, 0.0, 0.0), f(0.0, 0.0, -1.0)),  // XPositive, Deg0
        rm!(r(0.0, -1.0, 0.0), u(0.0, 0.0, -1.0), f(-1.0, 0.0, 0.0)), // YNegative, Deg0
        rm!(r(0.0, -1.0, 0.0), u(-1.0, 0.0, 0.0), f(0.0, 0.0, 1.0)),  // XNegative, Deg0
        rm!(r(0.0, -1.0, 0.0), u(0.0, 0.0, 1.0), f(1.0, 0.0, 0.0)),   // YPositive, Deg0
        rm!(r(0.0, 0.0, 1.0), u(1.0, 0.0, 0.0), f(0.0, -1.0, 0.0)),   // XPositive, Deg90
        rm!(r(1.0, 0.0, 0.0), u(0.0, 0.0, -1.0), f(0.0, -1.0, 0.0)),  // YNegative, Deg90
        rm!(r(0.0, 0.0, -1.0), u(-1.0, 0.0, 0.0), f(0.0, -1.0, 0.0)), // XNegative, Deg90
        rm!(r(-1.0, 0.0, 0.0), u(0.0, 0.0, 1.0), f(0.0, -1.0, 0.0)),  // YPositive, Deg90
        rm!(r(0.0, 0.0, -1.0), u(1.0, 0.0, 0.0), f(0.0, 1.0, 0.0)),   // XPositive, Deg270
        rm!(r(-1.0, 0.0, 0.0), u(0.0, 0.0, -1.0), f(0.0, 1.0, 0.0)),  // YNegative, Deg270
        rm!(r(0.0, 0.0, 1.0), u(-1.0, 0.0, 0.0), f(0.0, 1.0, 0.0)),   // XNegative, Deg270
        rm!(r(1.0, 0.0, 0.0), u(0.0, 0.0, 1.0), f(0.0, 1.0, 0.0)),    // YPositive, Deg270
        rm!(r(1.0, 0.0, 0.0), u(0.0, 1.0, 0.0), f(0.0, 0.0, -1.0)),   // ZPositive, Deg0
        rm!(r(0.0, 0.0, -1.0), u(0.0, 1.0, 0.0), f(-1.0, 0.0, 0.0)),  // ZPositive, Deg90
        rm!(r(-1.0, 0.0, 0.0), u(0.0, 1.0, 0.0), f(0.0, 0.0, 1.0)),   // ZPositive, Deg180
        rm!(r(0.0, 0.0, 1.0), u(0.0, 1.0, 0.0), f(1.0, 0.0, 0.0)),    // ZPositive, Deg270
        rm!(r(-1.0, 0.0, 0.0), u(0.0, -1.0, 0.0), f(0.0, 0.0, -1.0)), // ZNegative, Deg0
        rm!(r(0.0, 0.0, 1.0), u(0.0, -1.0, 0.0), f(-1.0, 0.0, 0.0)),  // ZNegative, Deg90
        rm!(r(1.0, 0.0, 0.0), u(0.0, -1.0, 0.0), f(0.0, 0.0, 1.0)),   // ZNegative, Deg180
        rm!(r(0.0, 0.0, -1.0), u(0.0, -1.0, 0.0), f(1.0, 0.0, 0.0)),  // ZNegative, Deg270
    ];
}

#[derive(Debug, Default, Clone, Copy)]
pub struct AABB {
    pub pos: IVec3,
    pub size: IVec3,
}

impl AABB {
    fn from_brick(brick: &Brick, save_data: &SaveData) -> Self {
        let pos = IVec3::new(
            brick.position.0 as i32,
            brick.position.2 as i32,
            brick.position.1 as i32,
        );
        let w = get_axis_size(brick, &save_data.header2.brick_assets, 0) as i32;
        let h = get_axis_size(brick, &save_data.header2.brick_assets, 2) as i32;
        let l = get_axis_size(brick, &save_data.header2.brick_assets, 1) as i32;
        Self {
            pos,
            size: IVec3::new(w, h, l),
        }
    }

    fn intersects(&self, other: &AABB) -> bool {
        if (self.pos.x - other.pos.x).abs() > (self.size.x + other.size.x) {
            return false;
        }
        if (self.pos.y - other.pos.y).abs() > (self.size.y + other.size.y) {
            return false;
        }
        if (self.pos.z - other.pos.z).abs() > (self.size.z + other.size.z) {
            return false;
        }
        true
    }
}

#[derive(Debug, Clone)]
pub struct BrickData {
    pub aabb: AABB,
    pub brick_index: usize,
    pub faces: Vec<Face>,
}

pub enum BVHNode {
    Leaf { data: BrickData },
    Internal { aabb: AABB, left: Box<BVHNode>, right: Box<BVHNode> }
}

pub fn construct_bvh(save_data: &SaveData) -> BVHNode {
    let now = SystemTime::now();
    let (bricks, total_faces) = build_faces(save_data);
    let bvh = top_down_bv_tree(bricks, 0);
    info!("Built {} faces and BVH in {} seconds", total_faces, now.elapsed().unwrap().as_secs_f32());
    bvh
}

pub fn gen_mesh(bvh: &BVHNode, save_data: &SaveData) -> Mesh {
    let mut bricks = Vec::with_capacity(save_data.bricks.len());
    brick_traverse(&bvh, &mut bricks, &bvh);

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

    let mut position_buffer: Vec<[f32; 3]> = Vec::new();
    let mut color_buffer: Vec<[f32; 4]> = Vec::new();
    let mut normal_buffer: Vec<[f32; 3]> = Vec::new();

    let mut final_faces = 0;

    for brick in bricks {
        let color = &save_data.bricks[brick.brick_index].color;
        let mut color = match color {
            BrickColor::Index(i) => cc(&save_data.header2.colors[*i as usize]),
            BrickColor::Unique(color) => cc(color),
        };

        for face in brick.faces {
            let positions = face.positions();
            let normal = face.normal.to_array();
            final_faces += 1;

            if face.color_override {
                color = [0.9, 0.08, 0.8, 1.0];
            }

            for pos in positions {
                position_buffer.push(pos);
                color_buffer.push(color);
                normal_buffer.push(normal);
            }
        }
    }

    info!("{} final faces", final_faces);

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, position_buffer);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, color_buffer);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normal_buffer);

    mesh
}

fn brick_traverse(bvh: &BVHNode, bricks: &mut Vec<BrickData>, root: &BVHNode) {
    match bvh {
        BVHNode::Internal { aabb: _, left, right } => {
            brick_traverse(left, bricks, root);
            brick_traverse(right, bricks, root);
        },
        BVHNode::Leaf { data } => {
            let mut brick = data.clone();

            let mut neighbors = vec![];
            traverse_neighbors(root, &brick, &mut neighbors);

            cull_faces(&mut brick, neighbors);

            bricks.push(brick.clone());
        }
    }
}

fn cull_faces(brick: &mut BrickData, neighbors: Vec<BrickData>) {
    let mut neighbor_faces: HashMap<IVec3, Vec<Face>> = HashMap::new();
    for mut neighbor in neighbors {
        while let Some(face) = neighbor.faces.pop() {
            let int_normal = (face.normal * 100.0).as_ivec3();
            if neighbor_faces.contains_key(&int_normal) {
                neighbor_faces.get_mut(&int_normal).unwrap().push(face);
            } else {
                neighbor_faces.insert(int_normal, vec![face]);
            }
        }
    }

    // brick.faces.retain(|face| {
    //     let int_normal = (face.normal * 100.0).as_ivec3();
    //     let aligned_faces = &neighbor_faces.get(&int_normal);
    //     if aligned_faces.is_none() {
    //         return true;
    //     }
    //     let aligned_faces = aligned_faces.unwrap();

    //     for other_face in aligned_faces {
    //         if face.inside(other_face) {
    //             return false;
    //         }
    //     }

    //     true
    // });

    for face in &mut brick.faces {
        let int_normal = (face.normal * 100.0).as_ivec3();
        let aligned_faces = &neighbor_faces.get(&int_normal);
        if aligned_faces.is_none() {
            continue;
        }
        let aligned_faces = aligned_faces.unwrap();

        for other_face in aligned_faces {
            if face.inside(other_face) {
                face.color_override = true;
            }
        }
    }
}

fn traverse_neighbors(bvh: &BVHNode, target: &BrickData, neighbors: &mut Vec<BrickData>) {
    match bvh {
        BVHNode::Internal { aabb: b, left, right } => {
            if !target.aabb.intersects(b) {
                return;
            }
            traverse_neighbors(&left, target, neighbors);
            traverse_neighbors(&right, target, neighbors);
        },
        BVHNode::Leaf { data: brick } => {
            if !target.aabb.intersects(&brick.aabb) || target.brick_index == brick.brick_index {
                return;
            }
            neighbors.push(brick.clone());
        }
    }
}

fn top_down_bv_tree(mut bricks: Vec<BrickData>, cut_axis: u8) -> BVHNode {
    if bricks.len() <= 1 {
        BVHNode::Leaf {
            data: bricks.pop().unwrap()
        }
    } else {
        let (k, aabb) = partition_bricks(&mut bricks, cut_axis);

        let right_bricks = bricks.drain(k..).collect();
        let left_bricks = bricks;

        let new_axis = (cut_axis + 1) % 3;

        let left = Box::new(top_down_bv_tree(left_bricks, new_axis));
        let right = Box::new(top_down_bv_tree(right_bricks, new_axis));

        BVHNode::Internal { aabb, left, right }
    }
}

fn partition_bricks(bricks: &mut Vec<BrickData>, cut_axis: u8) -> (usize, AABB) {
    match cut_axis {
        0 => {
            bricks.sort_by_key(|data| data.aabb.pos.x);
        },
        1 => {
            bricks.sort_by_key(|data| data.aabb.pos.y);
        },
        2 => {
            bricks.sort_by_key(|data| data.aabb.pos.z);
        },
        _ => unreachable!()
    }

    // calculate volume containing all sub-volumes
    let mut min = bricks[0].aabb.pos;
    let mut max = bricks[0].aabb.pos;

    for brick in bricks.iter() {
        let brick_min = brick.aabb.pos - brick.aabb.size;
        let brick_max = brick.aabb.pos + brick.aabb.size;

        min = min.min(brick_min);
        max = max.max(brick_max);
    }

    // if total size is uneven add a bit to the max of the volume
    let size: IVec3 = max - min;
    if size.x % 2 == 0 {
        max.x += 1;
    }
    if size.y % 2 == 0 {
        max.y += 1;
    }
    if size.z % 2 == 0 {
        max.z += 1;
    }

    let center = (min + max) / 2;
    let half_width = (max - min) / 2;

    let aabb = AABB {
        pos: center,
        size: half_width
    };

    
    (bricks.len() / 2, aabb)
}

fn build_faces(save_data: &SaveData) -> (Vec<BrickData>, usize) {
    let mut facecount = 0;
    let mut data = Vec::with_capacity(save_data.bricks.len());
    for i in 0..save_data.bricks.len() {
        let brick = &save_data.bricks[i];

        if !brick.visibility {
            continue;
        }

        let brick_asset = &save_data.header2.brick_assets[brick.asset_name_index as usize];
        let size = match brick.size {
            Size::Procedural(w, l, h) => Vec3::new(w as f32, h as f32, l as f32),
            Size::Empty => {
                if !BRICK_SIZE_MAP.contains_key(brick_asset.as_str()) {
                    continue;
                }
                let (w, l, h) = BRICK_SIZE_MAP[brick_asset.as_str()];
                Vec3::new(w as f32, h as f32, l as f32)
            }
        };

        let mut brick_faces = match brick_asset.as_str() {
            "PB_DefaultWedge" => default_wedge(size),
            "PB_DefaultRampInnerCorner" => ramp_inner_corner(size),
            "PB_DefaultRampCrest" => ramp_crest(size),
            "PB_DefaultRampCorner" => ramp_corner(size),
            "PB_DefaultMicroWedgeInnerCorner" => microwedge_inner_corner(size),
            "PB_DefaultMicroWedgeCorner" => microwedge_corner(size),
            "PB_DefaultMicroWedgeHalfOuterCorner" => microwedge_half_outer_corner(size),
            "PB_DefaultMicroWedgeHalfInnerCornerInverted" => microwedge_half_inner_corner_inverted(size),
            "PB_DefaultMicroWedgeHalfInnerCorner" => microwedge_half_inner_corner(size),
            "PB_DefaultMicroWedgeOuterCorner" => microwedge_outer_corner(size),
            "PB_DefaultMicroWedgeTriangleCorner" => microwedge_triangle_corner(size),
            "PB_DefaultRamp" => ramp(size),
            "PB_DefaultMicroWedge" | "PB_DefaultSideWedgeTile" | "PB_DefaultSideWedge" => side_wedge(size),
            _ => standard_brick(size)
        };

        for face in &mut brick_faces {
            for vert in &mut face.verts {
                *vert = ORIENTATION_MAP[d2o(brick.direction as u8, brick.rotation as u8) as usize]
                        .mul_vec3(*vert);
                *vert = *vert + brick_pos(brick);
            }
            face.calc_normal();
        }

        // cull downward faces
        brick_faces.retain(|face| {
            face.normal != Vec3::NEG_Y
        });

        facecount += brick_faces.len();

        data.push(BrickData {
            aabb: AABB::from_brick(brick, save_data),
            brick_index: i,
            faces: brick_faces
        });
    }
    (data, facecount)
}

fn brick_pos(brick: &Brick) -> Vec3 {
    Vec3::new(
        brick.position.0 as f32,
        brick.position.2 as f32,
        brick.position.1 as f32,
    )
}

