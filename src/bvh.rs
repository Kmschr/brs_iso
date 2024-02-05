use std::{ops::{Index, Neg}, time::SystemTime};

use bevy::{math::I64Vec3, prelude::*, render::render_resource::PrimitiveTopology, utils::{HashMap, HashSet}};
use rayon::prelude::*;
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

const CHUNK_SIZE: i32 = 2048;

// Arena Tree Bounding Volume Hierarchy
pub struct BVH {
    pub arena: Vec<BVHNode>,
    pub root: usize,
}

impl BVH {
    fn new(indices: Vec<usize>, aabbs: &Vec<AABB>) -> Self {
        let mut arena = Vec::with_capacity(indices.len() * 2);
        let root = top_down_bv_tree(indices, &aabbs, &mut arena);
        Self { arena, root }
    }
}

impl Index<usize> for BVH {
    type Output = BVHNode;

    fn index(&self, index: usize) -> &Self::Output {
        &self.arena[index]
    }
}

pub enum BVHNode {
    Leaf { i: usize },
    Internal { aabb: AABB, left: usize, right: usize }
}

pub struct Buffers {
    position: Vec<[f32; 3]>,
    color: Vec<[f32; 4]>,
    normal: Vec<[f32; 3]>
}

impl Buffers {
    fn new() -> Self {
        Self {
            position: Vec::new(),
            color: Vec::new(),
            normal: Vec::new()
        }
    }
}

pub struct BVHMeshGenerator<'a> {
    save_data: &'a SaveData,
    faces: Vec<Vec<Face>>,
    aabbs: Vec<AABB>,
    pub bvh: BVH,
}

impl<'a> BVHMeshGenerator<'a> {
    pub fn new(save_data: &'a SaveData) -> Self {
        let faces = gen_faces(save_data);
        let aabbs = gen_aabbs(save_data);
        let now = SystemTime::now();
        let indices = (0..save_data.bricks.len()).collect();
        let bvh = BVH::new(indices, &aabbs);
        info!("Built BVH in {} seconds", now.elapsed().unwrap().as_secs_f32());

        Self {
            save_data,
            faces,
            aabbs,
            bvh,
        }
    }

    pub fn gen_mesh(&self) -> Vec<Vec<Mesh>> {
        let now = SystemTime::now();
        let hidden: Vec<(usize, usize)> = self.save_data.bricks.par_iter().enumerate()
            .filter_map(|(i, _)| {
                if !&self.save_data.bricks[i].visibility || self.faces[i].len() == 0 {
                    None
                } else {
                    let mut neighbors = vec![];
                    self.traverse_neighbors(self.bvh.root, i, &mut neighbors);
                    Some(self.cull_faces(i, neighbors))
                }
            })
            .flatten()
            .collect();
        // convert vec to hashset for faster lookup
        let hidden: HashSet<(usize, usize)> = hidden.into_iter().collect();
        info!("Culled faces in {} seconds", now.elapsed().unwrap().as_secs_f32());

        let now = SystemTime::now();
        let mut material_chunks: Vec<HashMap<IVec3, Buffers>> = vec![
            HashMap::new(),
            HashMap::new(),
            HashMap::new(),
            HashMap::new(),
        ];

        let mut material_map: Vec<usize> = Vec::with_capacity(self.save_data.header2.materials.len());
        for i in 0..self.save_data.header2.materials.len() {
            let point = match self.save_data.header2.materials[i].as_str() {
                "BMC_Plastic" => 0,
                "BMC_Glow" => 1,
                "BMC_Glass" => 2,
                "BMC_Metallic" => 3,
                _ => 0,
            };
            material_map.push(point);
        }

        let mut final_faces = 0;
        for i in 0..self.save_data.bricks.len() {
            let brick_faces = &self.faces[i];
            if brick_faces.is_empty() {
                continue;
            }

            let material = material_map[self.save_data.bricks[i].material_index as usize];

            let chunk_coordinates = self.aabbs[i].center / CHUNK_SIZE;
            let chunks = &mut material_chunks[material];
            if !chunks.contains_key(&chunk_coordinates) {
                chunks.insert(chunk_coordinates.clone(), Buffers::new());
            }

            let buffers = chunks.get_mut(&chunk_coordinates).unwrap();

            let color = &self.save_data.bricks[i].color;
            let color = match color {
                BrickColor::Index(i) => cc(&self.save_data.header2.colors[*i as usize]),
                BrickColor::Unique(color) => cc(color),
            };
    
            for j in 0..brick_faces.len() {
                if hidden.contains(&(i, j)) {
                    continue;
                }

                let face = &self.faces[i][j];
                let positions = face.positions();
                let normal = face.normal.to_array();

                final_faces += 1;
    
                for pos in positions {
                    buffers.position.push(pos);
                    buffers.color.push(color);
                    buffers.normal.push(normal);
                }
            }
        }
    
        info!("{} final faces", final_faces);
    
        let mut material_meshes: Vec<Vec<Mesh>> = vec![
            Vec::with_capacity(material_chunks[0].len()),
            Vec::with_capacity(material_chunks[1].len()),
            Vec::with_capacity(material_chunks[2].len()),
            Vec::with_capacity(material_chunks[3].len()),
        ];

        let mut total_chunks = 0;
        let mut i = 0;
        for chunks in material_chunks.into_iter() {
            for (_, buffers) in chunks.into_iter() {
                let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
                mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, buffers.position);
                mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, buffers.color);
                mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, buffers.normal);
                material_meshes[i].push(mesh);
                total_chunks += 1;
            }
            i += 1;
        }

        info!("Generated {} mesh chunks in {} seconds", total_chunks, now.elapsed().unwrap().as_secs_f32());
        material_meshes
    }

    pub fn center_of_mass(&self) -> Vec3 {
        let total_mass: i64 = self.aabbs.iter().map(|aabb| aabb.volume()).sum();
        let weighted_sum = self.aabbs.iter().map(|aabb| aabb.center.as_i64vec3() * aabb.volume()).fold(I64Vec3::ZERO, |acc, val| acc + val);
        (weighted_sum / total_mass).as_vec3()
    }

    fn cull_faces(&self, target: usize, neighbors: Vec<usize>) -> Vec<(usize, usize)> {
        let mut hidden: Vec<(usize, usize)> = Vec::new();
        let mut neighbor_faces: HashMap<IVec3, Vec<(usize, usize)>> = HashMap::new();
        for i in neighbors {
            for j in 0..self.faces[i].len() {
                let face = &self.faces[i][j];
                let int_normal = (face.normal * 100.0).as_ivec3();
                neighbor_faces.entry(int_normal).or_insert_with(Vec::new).push((i, j));
            }
        }

        for j in 0..self.faces[target].len() {
            let face = &self.faces[target][j];
            let int_normal = (face.normal * 100.0).as_ivec3().neg();
            let opposite_faces = &neighbor_faces.get(&int_normal);
            if opposite_faces.is_none() {
                continue;
            }
            let coplanar_faces = opposite_faces.unwrap();
            for (other_i, other_j) in coplanar_faces {
                let other = &self.faces[*other_i][*other_j];
                if face.inside(other) {
                    hidden.push((target, j));
                    break;
                }
            }
        }

        hidden
    }
    

    fn traverse_neighbors(&self, current_node: usize, target_index: usize, neighbors: &mut Vec<usize>) {
        let target_aabb = self.aabbs[target_index];
        match &self.bvh[current_node] {
            BVHNode::Internal { aabb, left, right } => {
                if !target_aabb.intersects(aabb) {
                    return;
                }

                self.traverse_neighbors(*left, target_index, neighbors);
                self.traverse_neighbors(*right, target_index, neighbors);
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

fn top_down_bv_tree(mut brick_indices: Vec<usize>, aabbs: &Vec<AABB>, arena: &mut Vec<BVHNode>) -> usize {
    if brick_indices.len() <= 1 {
        let brick_index = brick_indices.pop().unwrap();
        arena.push(BVHNode::Leaf { i: brick_index });
    } else {
        let (k, aabb) = partition_bricks(&mut brick_indices, aabbs);

        let right_bricks = brick_indices.drain(k..).collect();
        let left_bricks = brick_indices;

        let left = top_down_bv_tree(left_bricks, aabbs, arena);
        let right = top_down_bv_tree(right_bricks, aabbs, arena);

        arena.push(BVHNode::Internal { aabb, left, right });
    }
    arena.len() - 1
}

fn partition_bricks(indices: &mut Vec<usize>, aabbs: &Vec<AABB>) -> (usize, AABB) {
    // calculate volume containing all sub-volumes
    let mut min = aabbs[indices[0]].center;
    let mut max = aabbs[indices[0]].center;

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

    // cut based on longest axis
    if aabb.halfwidths.x > aabb.halfwidths.y && aabb.halfwidths.x > aabb.halfwidths.z {
        indices.sort_unstable_by_key(|i| aabbs[*i].center.x);
    } else if aabb.halfwidths.y > aabb.halfwidths.x && aabb.halfwidths.y > aabb.halfwidths.z {
        indices.sort_unstable_by_key(|i| aabbs[*i].center.y);
    } else {
        indices.sort_unstable_by_key(|i| aabbs[*i].center.z);
    }
    
    (indices.len() / 2, aabb)
}


fn gen_faces(save_data: &SaveData) -> Vec<Vec<Face>> {
    let now = SystemTime::now();
    let mut data = Vec::with_capacity(save_data.bricks.len());

    data.par_extend(save_data.bricks.par_iter().map(|brick| {
        let mut brick_faces = Vec::new();

        if brick.visibility {
            let brick_asset = &save_data.header2.brick_assets[brick.asset_name_index as usize];
            let size = match brick.size {
                Size::Procedural(w, l, h) => Vec3::new(w as f32, h as f32, l as f32),
                Size::Empty => {
                    if let Some(&(w, l, h)) = BRICK_SIZE_MAP.get(brick_asset.as_str()) {
                        Vec3::new(w as f32, h as f32, l as f32)
                    } else {
                        return brick_faces;
                    }
                }
            };

            brick_faces = match brick_asset.as_str() {
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

            let brick_position = brick_pos(brick);
            for face in &mut brick_faces {
                for vert in &mut face.verts {
                    *vert = ORIENTATION_MAP[d2o(brick.direction as u8, brick.rotation as u8) as usize]
                            .mul_vec3(*vert);
                    *vert = *vert + brick_position;
                }
                face.calc_normal();
            }

            // cull downward faces
            brick_faces.retain(|face| {
                face.normal != Vec3::NEG_Y
            });
            // precalculate projection onto its normal plane
            for face in &mut brick_faces {
                face.calc_2d();
            }
        }

        brick_faces
    }));

    info!("Generated faces in {} seconds", now.elapsed().unwrap().as_secs_f32());

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
