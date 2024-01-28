use std::{time::SystemTime, ops::Neg, cell::RefCell};

use bevy::{prelude::*, render::render_resource::PrimitiveTopology, utils::HashMap};
use brickadia::{save::{SaveData, Size, Brick, BrickColor}, util::{BRICK_SIZE_MAP, rotation::d2o}};
use lazy_static::lazy_static;

use crate::{faces::*, aabb::AABB, utils::cc};

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

#[derive(Debug, Hash, PartialEq, Eq)]
struct ChunkCoordinates {
    x: i32,
    y: i32,
    z: i32,
}

struct BrickFaces(Option<Vec<Face>>);

pub enum BVHNode {
    Leaf { i: usize },
    Internal { aabb: AABB, left: Box<BVHNode>, right: Box<BVHNode> }
}

pub struct BVHMeshGenerator<'a> {
    save_data: &'a SaveData,
    faces: RefCell<Vec<BrickFaces>>,
    aabbs: Vec<AABB>,
    pub bvh: BVHNode,
}

impl<'a> BVHMeshGenerator<'a> {
    pub fn new(save_data: &'a SaveData) -> Self {
        let faces = RefCell::new(gen_faces(save_data));
        let aabbs = gen_aabbs(save_data);
        let now = SystemTime::now();
        let indices = (0..save_data.bricks.len()).collect();
        let bvh = top_down_bv_tree(indices, save_data, &aabbs, 0);
        info!("Built BVH in {} seconds", now.elapsed().unwrap().as_secs_f32());
        Self {
            save_data,
            faces,
            aabbs,
            bvh
        }
    }

    pub fn gen_mesh(&self) -> Mesh {
        let now = SystemTime::now();
        self.brick_traverse(&self.bvh);
        info!("Culled faces in {} seconds", now.elapsed().unwrap().as_secs_f32());
    
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    
        let mut position_buffer: Vec<[f32; 3]> = Vec::new();
        let mut color_buffer: Vec<[f32; 4]> = Vec::new();
        let mut normal_buffer: Vec<[f32; 3]> = Vec::new();
    
        let mut final_faces = 0;
    
        let faces = self.faces.borrow();
        for i in 0..self.save_data.bricks.len() {
            let color = &self.save_data.bricks[i].color;
            let color = match color {
                BrickColor::Index(i) => cc(&self.save_data.header2.colors[*i as usize]),
                BrickColor::Unique(color) => cc(color),
            };

            let brick_faces = faces[i].0.as_ref();
            if brick_faces.is_none() {
                continue;
            }
            let brick_faces = brick_faces.unwrap();
    
            for face in brick_faces {
                let positions = face.positions();
                let normal = face.normal.to_array();
    
                //let mut color = color;
                if face.hidden {
                    continue;
                    //color = [0.9, 0.08, 0.8, 1.0];
                }

                final_faces += 1;
    
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

    fn brick_traverse(&self, current_node: &BVHNode) {
        match current_node {
            BVHNode::Internal { aabb: _, left, right } => {
                self.brick_traverse(left);
                self.brick_traverse(right);
            },
            BVHNode::Leaf { i } => {
                if self.faces.borrow()[*i].0.is_none() {
                    return;
                }
                let mut neighbors = vec![];
                self.traverse_neighbors(&self.bvh, *i, &mut neighbors);    
                self.cull_faces(*i, neighbors);
            }
        }
    }

    fn cull_faces(&self, target: usize, neighbors: Vec<usize>) {
        let mut neighbor_faces: HashMap<IVec3, Vec<Face>> = HashMap::new();
        for neighbor in neighbors {
            let faces = &self.faces.borrow()[neighbor].0;
            if faces.is_none() {
                continue;
            }
            let faces = faces.as_ref().unwrap();

            for face in faces {
                let int_normal = (face.normal * 100.0).as_ivec3();
                if neighbor_faces.contains_key(&int_normal) {
                    neighbor_faces.get_mut(&int_normal).unwrap().push(face.clone());
                } else {
                    neighbor_faces.insert(int_normal, vec![face.clone()]);
                }
            }
        }

        let mut faces = self.faces.borrow_mut();
        let brick_faces = faces[target].0.as_mut().unwrap();
        for face in brick_faces {
            let int_normal = (face.normal * 100.0).as_ivec3().neg();
            let opposite_faces = &neighbor_faces.get(&int_normal);
            if opposite_faces.is_none() {
                continue;
            }
            let coplanar_faces = opposite_faces.unwrap();
            for other_face in coplanar_faces {
                if face.inside(other_face) {
                    face.hidden = true;
                    break;
                }
            }
        }
    }
    

    fn traverse_neighbors(&self, current_node: &BVHNode, target_index: usize, neighbors: &mut Vec<usize>) {
        let target_aabb = self.aabbs[target_index];
        match current_node {
            BVHNode::Internal { aabb, left, right } => {
                if !target_aabb.intersects(aabb) {
                    return;
                }
                self.traverse_neighbors(&left, target_index, neighbors);
                self.traverse_neighbors(&right, target_index, neighbors);
            },
            BVHNode::Leaf { i } => {
                if target_index == *i || !target_aabb.intersects(&self.aabbs[*i])  {
                    return;
                }
                neighbors.push(*i);
            }
        }
    }
}

fn top_down_bv_tree(mut indices: Vec<usize>, save_data: &SaveData, aabbs: &Vec<AABB>, cut_axis: u8) -> BVHNode {
    if indices.len() <= 1 {
        let i = indices.pop().unwrap();
        BVHNode::Leaf {
            i,
        }
    } else {
        let (k, aabb) = partition_bricks(&mut indices, aabbs, cut_axis);

        let right_bricks = indices.drain(k..).collect();
        let left_bricks = indices;

        let new_axis = (cut_axis + 1) % 3;

        let left = Box::new(top_down_bv_tree(left_bricks, save_data, aabbs, new_axis));
        let right = Box::new(top_down_bv_tree(right_bricks, save_data, aabbs, new_axis));

        BVHNode::Internal { aabb, left, right }
    }
}

fn partition_bricks(indices: &mut Vec<usize>, aabbs: &Vec<AABB>, cut_axis: u8) -> (usize, AABB) {
    match cut_axis {
        0 => {
            indices.sort_by_key(|i| aabbs[*i].center.x);
        },
        1 => {
            indices.sort_by_key(|i| aabbs[*i].center.y);
        },
        2 => {
            indices.sort_by_key(|i| aabbs[*i].center.z);
        },
        _ => unreachable!()
    }

    // calculate volume containing all sub-volumes
    let mut min = aabbs[0].center;
    let mut max = aabbs[0].center;

    for i in indices.iter() {
        let aabb = &aabbs[*i];
        let aabb_min = aabb.center - aabb.halfwidths;
        let aabb_max = aabb.center + aabb.halfwidths;

        min = min.min(aabb_min);
        max = max.max(aabb_max);
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
    let halfwidths = (max - min) / 2;

    let aabb = AABB {
        center,
        halfwidths
    };

    
    (indices.len() / 2, aabb)
}


fn gen_faces(save_data: &SaveData) -> Vec<BrickFaces> {
    let now = SystemTime::now();
    let mut facecount = 0;
    let mut data = Vec::with_capacity(save_data.bricks.len());
    for i in 0..save_data.bricks.len() {
        let brick = &save_data.bricks[i];

        if !brick.visibility {
            data.push(BrickFaces(None));
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

        data.push(BrickFaces(Some(brick_faces)));
    }

    info!("Generated {} faces in {} seconds", facecount, now.elapsed().unwrap().as_secs_f32());

    data
}

fn gen_aabbs(save_data: &SaveData) -> Vec<AABB> {
    let now = SystemTime::now();
    let mut aabbs = Vec::with_capacity(save_data.bricks.len());

    for brick in &save_data.bricks {
        aabbs.push(AABB::from_brick(brick, save_data));
    }

    info!("Generated AABBs in {} seconds", now.elapsed().unwrap().as_secs_f32());

    aabbs
}

fn brick_pos(brick: &Brick) -> Vec3 {
    Vec3::new(
        brick.position.0 as f32,
        brick.position.2 as f32,
        brick.position.1 as f32,
    )
}
