use std::hash::Hasher;
use std::hash::Hash;

use bevy::utils::AHasher;
use bevy::utils::HashMap;
use bevy::utils::HashSet;
use bevy::{
    prelude::*,
    render::render_resource::PrimitiveTopology,
};
use brickadia::util::BRICK_SIZE_MAP;
use brickadia::{
    save::{BrickColor, Color, SaveData, Size, Brick},
    util::{rotation::d2o, octree::CHUNK_SIZE},
};
use lazy_static::lazy_static;

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

const A: Vec3 = Vec3::new(-1., 1., 1.);
const B: Vec3 = Vec3::new(-1., 1., -1.);
const C: Vec3 = Vec3::new(1., 1., -1.);
const D: Vec3 = Vec3::new(1., 1., 1.);
const E: Vec3 = Vec3::new(-1., -1., 1.);
const F: Vec3 = Vec3::new(-1., -1., -1.);
const G: Vec3 = Vec3::new(1., -1., -1.);
const H: Vec3 = Vec3::new(1., -1., 1.);
const I: Vec3 = Vec3::new(0., 1., 1.);
const J: Vec3 = Vec3::new(0., 1., -1.);
const TENX: Vec3 = Vec3::new(10.0, 0.0, 0.0);
const TENZ: Vec3 = Vec3::new(0.0, 0.0, 10.0);
const TWOY: Vec3 = Vec3::new(0.0, 2.0, 0.0);

//        B          C
//         +---J----+           
//        /        /|            
//       /      D / |           
//    A +----I---+  |            
//      |        |  |           
//      |    F   |  +  G         
//      |        | /              
//      |        |/               
//      +--------+         
//    E            H       
//
//
//  
//    Y
//   ^     -Z
//   |   /
//   |  /
//   | /
//   |/
//   +---------> X
//  

#[derive(Debug, Hash, PartialEq, Eq)]
struct ChunkCoordinates {
    x: i32,
    y: i32,
    z: i32,
}

#[derive(Debug)]
struct Chunk {
    bricks: Vec<Brick>
}

#[derive(Debug, Default, Clone)]
struct Face {
    // verts start at top left corner of face and are ordered clockwise
    verts: Vec<Vec3>,
    normal: Vec3,
    color: [f32; 4]
}

impl Face {
    fn new(verts: Vec<Vec3>) -> Self {
        Face {
            verts,
            ..default()
        }
    }

    fn calc_normal(&mut self) {
        if self.verts.len() < 3 {
            error!("Can't calculate normal with less than 3 vertices");
            return;
        }

        let a = self.verts[0];
        let b = self.verts[1];
        let c = self.verts[2];

        let dir = (b - a).cross(c - a);
        let normal = dir / -dir.length();

        self.normal = normal;
    }

    fn positions(&self) -> Vec<[f32; 3]> {
        let mut positions = vec![];
        for i in 0..(self.verts.len() - 2) {
            positions.push(self.verts[0].to_array());
            positions.push(self.verts[2 + i].to_array());
            positions.push(self.verts[1 + i].to_array());
        }
        positions
    }
}

impl Hash for Face {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut verts: Vec<u128> = Vec::with_capacity(self.verts.len());
        for v in &self.verts {
            let x_bits = v.x.to_bits() as u128;
            let y_bits = v.y.to_bits() as u128;
            let z_bits = v.z.to_bits() as u128;

            let value = (x_bits << 64) | (y_bits << 32) | z_bits;

            verts.push(value);
        }
        verts.sort();
        for vert in verts {
            vert.hash(state);
        }
    }
}

impl PartialEq for Face {
    fn eq(&self, other: &Self) -> bool {
        let mut hasher = AHasher::default();
        self.hash(&mut hasher);
        let a = hasher.finish();

        let mut hasher = AHasher::default();
        other.hash(&mut hasher);
        let b = hasher.finish();

        a == b
    }
}

impl Eq for Face {

}

pub fn gen_save_mesh(save_data: &SaveData, brick_type: &str) -> Vec<Mesh> {
    let mut chunks: HashMap<ChunkCoordinates, Chunk> = HashMap::default();

    for brick in &save_data.bricks {
        if save_data.header2.materials[brick.material_index as usize] != brick_type ||
            !brick.visibility {
            continue;
        }

        let coord = ChunkCoordinates {
            x: brick.position.0 / CHUNK_SIZE,
            y: brick.position.1 / CHUNK_SIZE,
            z: brick.position.2 / CHUNK_SIZE,
        };

        if let Some(chunk) = chunks.get_mut(&coord) {
            chunk.bricks.push(brick.clone());
        } else {
            chunks.insert(coord, Chunk {
                bricks: vec![brick.clone()]
            });
        }
    }

    let mut pre_coverage_count = 0;
    let mut final_face_count = 0;

    let color_palette = &save_data
        .header2
        .colors
        .iter()
        .map(|color| cc(&color))
        .collect::<Vec<[f32; 4]>>();

    let mut meshes = vec![];
    for (_coords, chunk) in chunks.iter() {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

        let mut position_buffer: Vec<[f32; 3]> = Vec::new();
        let mut color_buffer: Vec<[f32; 4]> = Vec::new();
        let mut normal_buffer: Vec<[f32; 3]> = Vec::new();

        let mut chunk_faces: Vec<Face> = vec![];

        for brick in &chunk.bricks {
            let brick_asset = &save_data.header2.brick_assets[brick.asset_name_index as usize];

            let size = match brick.size {
                Size::Procedural(w, l, h) => Vec3::new(w as f32, h as f32, l as f32),
                Size::Empty => {
                    let (w, l, h) = BRICK_SIZE_MAP[brick_asset.as_str()];
                    Vec3::new(w as f32, h as f32, l as f32)
                }
            };
            
            let mut brick_faces = match brick_asset.as_str() {
                "PB_DefaultWedge" => {
                    vec![
                        Face::new(vec![
                            size * A,
                            size * B,
                            size * G + TWOY,
                            size * H + TWOY,
                        ]),
                        Face::new(vec![
                            size * A,
                            size * H + TWOY,
                            size * H,
                            size * E,
                        ]),
                        Face::new(vec![
                            size * H + TWOY,
                            size * G + TWOY,
                            size * G,
                            size * H,
                        ]),
                        Face::new(vec![
                            size * B,
                            size * A,
                            size * E,
                            size * F,
                        ]),
                        Face::new(vec![
                            size * B,
                            size * F,
                            size * G,
                            size * G + TWOY,
                        ]),
                        Face::new(vec![
                            size * E,
                            size * H,
                            size * G,
                            size * F,
                        ]),
                    ]
                },
                "PB_DefaultRampInnerCorner" => {
                    vec![
                        Face::new(vec![
                            size * A,
                            size * B,
                            size * B + TENX,
                            size * A + TENX,
                        ]),
                        Face::new(vec![
                            size * B + TENX,
                            size * C,
                            size * C + TENZ,
                            size * B + TENX + TENZ,
                        ]),
                        Face::new(vec![
                            size * A + TENX,
                            size * B + TENX + TENZ,
                            size * H + TWOY,
                        ]),
                        Face::new(vec![
                            size * B + TENX + TENZ,
                            size * C + TENZ,
                            size * H + TWOY,
                        ]),
                        Face::new(vec![
                            size * A,
                            size * A + TENX,
                            size * H + TWOY,
                            size * H,
                            size * E,
                        ]),
                        Face::new(vec![
                            size * C + TENZ,
                            size * C,
                            size * G,
                            size * H,
                            size * H + TWOY,
                        ]),
                        Face::new(vec![
                            size * B,
                            size * A,
                            size * E,
                            size * F,
                        ]),
                        Face::new(vec![
                            size * C,
                            size * B,
                            size * F,
                            size * G,
                        ]),
                        Face::new(vec![
                            size * E,
                            size * H,
                            size * G,
                            size * F,
                        ]),
                    ]
                }
                "PB_DefaultRampCrest" => {
                    vec![
                        Face::new(vec![
                            size * I,
                            size * H + TWOY,
                            size * H,
                            size * E,
                            size * E + TWOY,
                        ]),
                        Face::new(vec![
                            size * F + TWOY,
                            size * E + TWOY,
                            size * E,
                            size * F,
                        ]),
                        Face::new(vec![
                            size * H + TWOY,
                            size * G + TWOY,
                            size * G,
                            size * H,
                        ]),
                        Face::new(vec![
                            size * J,
                            size * I,
                            size * E + TWOY,
                            size * F + TWOY,
                        ]),
                        Face::new(vec![
                            size * I,
                            size * J,
                            size * G + TWOY,
                            size * H + TWOY,
                        ]),
                        Face::new(vec![
                            size * J,
                            size * F + TWOY,
                            size * F,
                            size * G,
                            size * G + TWOY,
                        ]),
                        Face::new(vec![
                            size * E,
                            size * H,
                            size * G,
                            size * F,
                        ]),
                    ]
                },
                "PB_DefaultRampCorner" => {
                    vec![
                        Face::new(vec![
                            size * B,
                            size * B + TENX,
                            size * B + TENX + TENZ,
                            size * B + TENZ,
                        ]),
                        Face::new(vec![
                            size * B + TENZ,
                            size * B + TENX + TENZ,
                            size * H + TWOY,
                            size * E + TWOY,
                        ]),
                        Face::new(vec![
                            size * B + TENX + TENZ,
                            size * B + TENX,
                            size * G + TWOY,
                            size * H + TWOY,
                        ]),
                        Face::new(vec![
                            size * E + TWOY,
                            size * H + TWOY,
                            size * H,
                            size * E,
                        ]),
                        Face::new(vec![
                            size * H + TWOY,
                            size * G + TWOY,
                            size * G,
                            size * H,
                        ]),
                        Face::new(vec![
                            size * B,
                            size * B + TENZ,
                            size * E + TWOY,
                            size * E,
                            size * F,
                        ]),
                        Face::new(vec![
                            size * B + TENX,
                            size * B,
                            size * F,
                            size * G,
                            size * G + TWOY,
                        ]),
                        Face::new(vec![
                            size * E,
                            size * H,
                            size * G,
                            size * F,
                        ]),
                    ]
                },
                "PB_DefaultMicroWedgeInnerCorner" => {
                    vec![
                        Face::new(vec![
                            size * B,
                            size * H,
                            size * A,
                        ]),
                        Face::new(vec![
                            size * B,
                            size * C,
                            size * H,
                        ]),
                        Face::new(vec![
                            size * A,
                            size * H,
                            size * E,
                        ]),
                        Face::new(vec![
                            size * B,
                            size * A,
                            size * E,
                            size * F,
                        ]),
                        Face::new(vec![
                            size * C,
                            size * G,
                            size * H,
                        ]),
                        Face::new(vec![
                            size * C,
                            size * B,
                            size * F,
                            size * G,
                        ]),
                        Face::new(vec![
                            size * E,
                            size * H,
                            size * G,
                            size * F,
                        ]),
                    ]
                },
                "PB_DefaultMicroWedgeCorner" => {
                    vec![
                        Face::new(vec![
                            size * B,
                            size * H,
                            size * E,
                        ]),
                        Face::new(vec![
                            size * B,
                            size * G,
                            size * H,
                        ]),
                        Face::new(vec![
                            size * B,
                            size * E,
                            size * F,
                        ]),
                        Face::new(vec![
                            size * B,
                            size * F,
                            size * G,
                        ]),
                        Face::new(vec![
                            size * E,
                            size * H,
                            size * G,
                            size * F,
                        ]),
                    ]
                }
                "PB_DefaultMicroWedgeHalfOuterCorner" => {
                    vec![
                        Face::new(vec![
                            size * A,
                            size * C,
                            size * G,
                            size * E,
                        ]),
                        Face::new(vec![
                            size * C,
                            size * A,
                            size * F,
                        ]),
                        Face::new(vec![
                            size * A,
                            size * E,
                            size * F,
                        ]),
                        Face::new(vec![
                            size * C,
                            size * F,
                            size * G,
                        ]),
                        Face::new(vec![
                            size * E,
                            size * G,
                            size * F,
                        ]),
                    ]
                }
                "PB_DefaultMicroWedgeHalfInnerCornerInverted" => {
                    vec![
                        Face::new(vec![
                            size * C,
                            size * E,
                            size * F,
                        ]),
                        Face::new(vec![
                            size * C,
                            size * G,
                            size * E,
                        ]),
                        Face::new(vec![
                            size * C,
                            size * F,
                            size * G,
                        ]),
                        Face::new(vec![
                            size * E,
                            size * G,
                            size * F,
                        ]),
                    ]
                },
                "PB_DefaultMicroWedgeHalfInnerCorner" => {
                    vec![
                        Face::new(vec![
                            size * A,
                            size * G,
                            size * E,
                        ]),
                        Face::new(vec![
                            size * A,
                            size * E,
                            size * F,
                        ]),
                        Face::new(vec![
                            size * F,
                            size * G,
                            size * A,
                        ]),
                        Face::new(vec![
                            size * E,
                            size * G,
                            size * F,
                        ]),
                    ]
                },
                "PB_DefaultMicroWedgeOuterCorner" => {
                    vec![
                        Face::new(vec![
                            size * A,
                            size * C,
                            size * H,
                        ]),
                        Face::new(vec![
                            size * B,
                            size * C,
                            size * A,
                        ]),
                        Face::new(vec![
                            size * A,
                            size * H,
                            size * E,
                        ]),
                        Face::new(vec![
                            size * C,
                            size * G,
                            size * H,
                        ]),
                        Face::new(vec![
                            size * C,
                            size * B,
                            size * F,
                            size * G,
                        ]),
                        Face::new(vec![
                            size * B,
                            size * A,
                            size * E,
                            size * F,
                        ]),
                        Face::new(vec![
                            size * E,
                            size * H,
                            size * G,
                            size * F,
                        ]),
                    ]
                },
                "PB_DefaultMicroWedgeTriangleCorner" => {
                    vec![
                        Face::new(vec![
                            size * B,
                            size * E,
                            size * F,
                        ]),
                        Face::new(vec![
                            size * B,
                            size * G,
                            size * E,
                        ]),
                        Face::new(vec![
                            size * B,
                            size * F,
                            size * G,
                        ]),
                        Face::new(vec![
                            size * F,
                            size * E,
                            size * G,
                        ]),
                    ]
                },
                "PB_DefaultRamp" => {
                    vec![
                        Face::new(vec![
                            size * B,
                            size * B + TENX,
                            size * A + TENX,
                            size * A,
                        ]),
                        Face::new(vec![
                            size * E,
                            size * H,
                            size * G,
                            size * F,
                        ]),
                        Face::new(vec![
                            size * A,
                            size * A + TENX,
                            size * H + TWOY,
                            size * H,
                            size * E,
                        ]),
                        Face::new(vec![
                            size * B + TENX,
                            size * B,
                            size * F,
                            size * G,
                            size * G + TWOY,
                        ]),
                        Face::new(vec![
                            size * B,
                            size * A,
                            size * E,
                            size * F,
                        ]),
                        Face::new(vec![
                            size * A + TENX,
                            size * B + TENX,
                            size * G + TWOY,
                            size * H + TWOY,
                        ]),
                        Face::new(vec![
                            size * H + TWOY,
                            size * G + TWOY,
                            size * G,
                            size * H,
                        ]),
                    ]
                },
                "PB_DefaultMicroWedge" | "PB_DefaultSideWedgeTile" | "PB_DefaultSideWedge" => {
                    vec![
                        Face::new(vec![
                            size * B,
                            size * C,
                            size * A,
                        ]),
                        Face::new(vec![
                            size * E,
                            size * G,
                            size * F,
                        ]),
                        Face::new(vec![
                            size * A,
                            size * C,
                            size * G,
                            size * E,
                        ]),
                        Face::new(vec![
                            size * C,
                            size * B,
                            size * F,
                            size * G,
                        ]),
                        Face::new(vec![
                            size * B,
                            size * A,
                            size * E,
                            size * F,
                        ]),
                    ]
                },
                _ => {
                    vec![
                        Face::new(vec![
                            size * B,
                            size * C,
                            size * D,
                            size * A,
                        ]),
                        Face::new(vec![
                            size * E,
                            size * H,
                            size * G,
                            size * F,
                        ]),
                        Face::new(vec![
                            size * A,
                            size * D,
                            size * H,
                            size * E,
                        ]),
                        Face::new(vec![
                            size * C,
                            size * B,
                            size * F,
                            size * G,
                        ]),
                        Face::new(vec![
                            size * B,
                            size * A,
                            size * E,
                            size * F,
                        ]),
                        Face::new(vec![
                            size * D,
                            size * C,
                            size * G,
                            size * H,
                        ]),
                    ]
                }
            };
            

            let translation = Vec3::new(
                brick.position.0 as f32,
                brick.position.2 as f32,
                brick.position.1 as f32,
            );

            let color = match &brick.color {
                BrickColor::Index(i) => color_palette[*i as usize],
                BrickColor::Unique(color) => cc(color),
            };

            for face in &mut brick_faces {
                for vert in &mut face.verts {
                    *vert = ORIENTATION_MAP[d2o(brick.direction as u8, brick.rotation as u8) as usize]
                            .mul_vec3(*vert);
                    *vert = *vert + translation;
                }
                face.calc_normal();
                face.color = color;

                if face.normal.y == -1.0 {
                    continue;
                }

                chunk_faces.push(face.clone());
            }
        }

        pre_coverage_count += chunk_faces.len();

        let mut face_set: HashSet<Face> = HashSet::new();
        while !chunk_faces.is_empty() {
            let face = chunk_faces.pop().unwrap();

            if face_set.contains(&face) {
                face_set.remove(&face);
            } else {
                face_set.insert(face);
            }
        }

        final_face_count += face_set.len();

        for face in face_set.iter() {
            let positions = face.positions();
            let normal = face.normal.to_array();

            for pos in positions {
                position_buffer.push(pos);
                color_buffer.push(face.color);
                normal_buffer.push(normal);
            }
        }

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, position_buffer);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, color_buffer);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normal_buffer);
        meshes.push(mesh);
    }

    info!("{} total faces", final_face_count);
    info!("{} faces removed by coverage", pre_coverage_count - final_face_count);

    meshes
}

pub fn cc(c: &Color) -> [f32; 4] {
    [
        c.r as f32 / 255.0,
        c.g as f32 / 255.0,
        c.b as f32 / 255.0,
        0.0,
    ]
}
