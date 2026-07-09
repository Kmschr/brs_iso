use std::{ops::{Index, Neg}, time::SystemTime};

use bevy::{asset::RenderAssetUsages, math::I64Vec3, mesh::{Indices, MeshVertexAttribute, VertexAttributeValues}, platform::collections::HashMap, prelude::*, render::render_resource::{PrimitiveTopology, VertexFormat}};
use rayon::prelude::*;
use brickadia::{save::{SaveData, Size, Brick, BrickColor}, util::{BRICK_SIZE_MAP, rotation::d2o}};
use lazy_static::lazy_static;

use crate::{faces::*, aabb::AABB, utils::cu8};

// Packed vertex attributes: 20 B/vertex instead of 40 B. These reuse the ids
// of `Mesh::ATTRIBUTE_NORMAL`/`ATTRIBUTE_COLOR`, so the standard PBR pipeline
// resolves the shader defs and vertex layout for them by id and takes the
// packed format from the mesh itself. The GPU unpacks to float during vertex
// fetch, and wgpu permits a 4-component format on the shader's vec3 normal
// input (narrowing is valid).
const ATTRIBUTE_PACKED_NORMAL: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Normal", 1, VertexFormat::Snorm8x4);
const ATTRIBUTE_PACKED_COLOR: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Color", 5, VertexFormat::Unorm8x4);

fn pack_normal(normal: Vec3) -> [i8; 4] {
    let q = (normal * 127.0).round();
    [q.x as i8, q.y as i8, q.z as i8, 0]
}

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
}

impl BVH {
    fn new(mut indices: Vec<usize>, aabbs: &[AABB]) -> Self {
        let mut bvh = Self {
            arena: Vec::with_capacity(indices.len() * 2)
        };
        if !indices.is_empty() {
            bvh.top_down_bv_tree(&mut indices, aabbs);
        }
        bvh
    }

    fn top_down_bv_tree(&mut self, brick_indices: &mut [usize], aabbs: &[AABB]) -> usize {
        let i = self.arena.len();
        if let [brick_index] = *brick_indices {
            self.arena.push(BVHNode::Leaf { i: brick_index });
        } else {
            let (k, aabb) = partition_bricks(brick_indices, aabbs);
            self.arena.push(BVHNode::Internal { aabb, left: 0, right: 0 });

            let (left_bricks, right_bricks) = brick_indices.split_at_mut(k);
            let left_idx = self.top_down_bv_tree(left_bricks, aabbs);
            let right_idx = self.top_down_bv_tree(right_bricks, aabbs);

            if let BVHNode::Internal { left, right, .. } = &mut self.arena[i] {
                *left = left_idx;
                *right = right_idx;
            }
        }
        i
    }

    pub fn intersection(&self, ray: Ray3d, aabbs: &Vec<AABB>) -> Option<usize> {
        let mut stack = vec![0];
        while let Some(node) = stack.pop() {
            match &self.arena[node] {
                BVHNode::Internal { aabb, left, right } => {
                    if aabb.intersects(ray) {
                        stack.push(*left);
                        stack.push(*right);
                    }
                },
                BVHNode::Leaf { i } => {
                    if aabbs[*i].intersects(ray) {
                        return Some(*i);
                    }
                }
            }
        }
        None
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

fn partition_bricks(indices: &mut [usize], aabbs: &[AABB]) -> (usize, AABB) {
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

    // split at the median along the longest axis; a full sort is wasted work
    // when only the median partition is needed
    let k = indices.len() / 2;
    let hw = aabb.halfwidths;
    if hw.x >= hw.y && hw.x >= hw.z {
        indices.select_nth_unstable_by_key(k, |i| aabbs[*i].center.x);
    } else if hw.y >= hw.z {
        indices.select_nth_unstable_by_key(k, |i| aabbs[*i].center.y);
    } else {
        indices.select_nth_unstable_by_key(k, |i| aabbs[*i].center.z);
    }

    (k, aabb)
}

pub struct Buffers {
    position: Vec<[f32; 3]>,
    color: Vec<[u8; 4]>,
    normal: Vec<[i8; 4]>,
    indices: Vec<u32>,
}

impl Buffers {
    fn new() -> Self {
        Self {
            position: Vec::new(),
            color: Vec::new(),
            normal: Vec::new(),
            indices: Vec::new(),
        }
    }
}

pub struct BVHMeshGenerator<'a> {
    save_data: &'a SaveData,
    faces: Vec<Vec<Face>>,
    pub aabbs: Vec<AABB>,
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
        // Hidden faces as a bitmask per brick (bricks have at most 9 faces).
        // map_init reuses the neighbor scratch buffers per rayon worker instead
        // of reallocating them for every brick.
        let hidden_masks: Vec<u16> = self.save_data.bricks.par_iter().enumerate()
            .map_init(
                || (Vec::new(), HashMap::default()),
                |(neighbors, neighbor_faces), (i, brick)| {
                    if !brick.visibility || self.faces[i].is_empty() {
                        return 0;
                    }
                    neighbors.clear();
                    self.traverse_neighbors(i, neighbors);
                    self.cull_faces(i, neighbors, neighbor_faces)
                },
            )
            .collect();
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
            let buffers = material_chunks[material]
                .entry(chunk_coordinates)
                .or_insert_with(Buffers::new);

            let color = &self.save_data.bricks[i].color;
            let color = match color {
                BrickColor::Index(i) => cu8(&self.save_data.header2.colors[*i as usize]),
                BrickColor::Unique(color) => cu8(color),
            };

            for j in 0..brick_faces.len() {
                if hidden_masks[i] & (1 << j) != 0 {
                    continue;
                }

                let face = &self.faces[i][j];
                let normal = pack_normal(face.normal);

                final_faces += 1;

                // Push each face's verts once and fan-triangulate with an index
                // buffer, instead of duplicating shared verts per triangle.
                let base = buffers.position.len() as u32;
                for vert in &face.verts {
                    buffers.position.push(vert.to_array());
                    buffers.color.push(color);
                    buffers.normal.push(normal);
                }
                for k in 0..(face.verts.len() as u32).saturating_sub(2) {
                    buffers.indices.push(base);
                    buffers.indices.push(base + 2 + k);
                    buffers.indices.push(base + 1 + k);
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
                // RENDER_WORLD only: nothing reads these meshes back on the
                // CPU (picking uses the BVH), so don't keep a main-world copy.
                let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
                mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, buffers.position);
                mesh.insert_attribute(ATTRIBUTE_PACKED_COLOR, VertexAttributeValues::Unorm8x4(buffers.color));
                mesh.insert_attribute(ATTRIBUTE_PACKED_NORMAL, VertexAttributeValues::Snorm8x4(buffers.normal));
                mesh.insert_indices(Indices::U32(buffers.indices));
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

    fn cull_faces(
        &self,
        target: usize,
        neighbors: &[usize],
        neighbor_faces: &mut HashMap<IVec3, Vec<(usize, usize)>>,
    ) -> u16 {
        neighbor_faces.clear();
        for &i in neighbors {
            for (j, face) in self.faces[i].iter().enumerate() {
                neighbor_faces.entry(face.int_normal).or_default().push((i, j));
            }
        }

        let mut hidden = 0u16;
        for (j, face) in self.faces[target].iter().enumerate() {
            let Some(coplanar_faces) = neighbor_faces.get(&face.int_normal.neg()) else {
                continue;
            };
            for &(other_i, other_j) in coplanar_faces {
                let other = &self.faces[other_i][other_j];
                if face.inside(other) {
                    hidden |= 1 << j;
                    break;
                }
            }
        }

        hidden
    }
    

    fn traverse_neighbors(&self, target_index: usize, neighbors: &mut Vec<usize>) {
        let target_aabb = self.aabbs[target_index];
        let mut stack = vec![0];
    
        while let Some(node) = stack.pop() {
            match &self.bvh[node] {
                BVHNode::Internal { aabb, left, right } => {
                    if target_aabb.neighbors(aabb) {
                        stack.push(*left);
                        stack.push(*right);
                    }
                },
                BVHNode::Leaf { i } => {
                    if target_index != *i && target_aabb.neighbors(&self.aabbs[*i]) {
                        neighbors.push(*i);
                    }
                }
            }
        }
    }
}

fn gen_faces(save_data: &SaveData) -> Vec<Vec<Face>> {
    let now = SystemTime::now();

    // Resolve the shape constructor and fixed size once per asset instead of
    // string-matching per brick.
    let asset_shapes: Vec<(fn(Vec3) -> Vec<Face>, Option<Vec3>)> = save_data.header2.brick_assets.iter()
        .map(|asset| {
            let shape_fn: fn(Vec3) -> Vec<Face> = match asset.as_str() {
                "PB_DefaultWedge" => default_wedge,
                "PB_DefaultRampInnerCorner" => ramp_inner_corner,
                "PB_DefaultRampCrest" => ramp_crest,
                "PB_DefaultRampCorner" => ramp_corner,
                "PB_DefaultMicroWedgeInnerCorner" => microwedge_inner_corner,
                "PB_DefaultMicroWedgeCorner" => microwedge_corner,
                "PB_DefaultMicroWedgeHalfOuterCorner" => microwedge_half_outer_corner,
                "PB_DefaultMicroWedgeHalfInnerCornerInverted" => microwedge_half_inner_corner_inverted,
                "PB_DefaultMicroWedgeHalfInnerCorner" => microwedge_half_inner_corner,
                "PB_DefaultMicroWedgeOuterCorner" => microwedge_outer_corner,
                "PB_DefaultMicroWedgeTriangleCorner" => microwedge_triangle_corner,
                "PB_DefaultRamp" => ramp,
                "PB_DefaultMicroWedge" | "PB_DefaultSideWedgeTile" | "PB_DefaultSideWedge" => side_wedge,
                _ => standard_brick,
            };
            let fixed_size = BRICK_SIZE_MAP.get(asset.as_str())
                .map(|&(w, l, h)| Vec3::new(w as f32, h as f32, l as f32));
            (shape_fn, fixed_size)
        })
        .collect();

    let mut data = Vec::with_capacity(save_data.bricks.len());

    data.par_extend(save_data.bricks.par_iter().map(|brick| {
        let mut brick_faces = Vec::new();

        if brick.visibility {
            let (shape_fn, fixed_size) = &asset_shapes[brick.asset_name_index as usize];
            let size = match brick.size {
                Size::Procedural(w, l, h) => Vec3::new(w as f32, h as f32, l as f32),
                Size::Empty => match fixed_size {
                    Some(size) => *size,
                    None => return brick_faces,
                }
            };

            brick_faces = shape_fn(size);

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
    aabbs.par_extend(save_data.bricks.par_iter().map(|brick| AABB::from_brick(brick, save_data)));
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
