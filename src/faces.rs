use std::{hash::Hasher, ops::Neg};
use std::hash::Hash;

use bevy::{prelude::*, utils::AHasher};

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
const TENX: Vec3 = Vec3::new(10., 0., 0.);
const TENZ: Vec3 = Vec3::new(0., 0., 10.);
const TWOY: Vec3 = Vec3::new(0., 2., 0.);

//           
//         B----J----C          
//        /|        /|            
//       / |       / |           
//      A----I----D  |            
//      |  |      |  |           
//      |  F------|--G          
//      | /       | /              
//      |/        |/               
//      E---------H         
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

#[derive(Debug, Default, Clone)]
pub struct Face {
    // verts start at top left corner of face and are ordered clockwise
    pub verts: Vec<Vec3>,
    pub normal: Vec3,
}

impl Face {
    pub fn new(verts: Vec<Vec3>) -> Self {
        Face {
            verts,
            ..default()
        }
    }

    pub fn calc_normal(&mut self) {
        assert!(self.verts.len() >= 3);
        let a = self.verts[0];
        let b = self.verts[1];
        let c = self.verts[2];

        let dir = (b - a).cross(c - a);
        let normal = -dir.normalize();

        let normal = ((normal * 10.).as_ivec3().as_vec3() / 10.).normalize();

        self.normal = normal;
    }

    pub fn positions(&self) -> Vec<[f32; 3]> {
        let mut positions = vec![];
        for i in 0..(self.verts.len() - 2) {
            positions.push(self.verts[0].to_array());
            positions.push(self.verts[2 + i].to_array());
            positions.push(self.verts[1 + i].to_array());
        }
        positions
    }

    pub fn to_2d(&self, flip_normal: bool) -> Vec<Vec2> {
        let mut points = Vec::with_capacity(self.verts.len());

        for p in &self.verts {
            let mut n = self.normal.clone();

            if flip_normal {
                n = n.neg();
            }

            let v = Vec3::new(666.0, 69.0, 420.0);

            // Calculate the cross product U = N X V
            let u = n.cross(v);

            // Normalize V and U
            let u = u.normalize();
            let v = v.normalize();

            let x = u.dot(*p);
            let y = v.dot(*p);

            points.push(Vec2::new(x, y));
        }

        points
    }

    pub fn inside(&self, other: &Face) -> bool {
        // check opposite coplanar and coincident
        if self.normal != other.normal.neg() || !self.coincident_planes(other) {
            return false;
        }

        for vert in self.to_2d(false) {
            if !point_inside_face(vert, other.to_2d(true)) {
                return false;
            }
        }

        true
    }

    pub fn coincident_planes(&self, other: &Face) -> bool {
        let distance = (self.verts[0] - other.verts[0]).dot(self.normal);
        if distance.abs() > 0.0001 {
            return false;
        }
        return true;
    }

    pub fn merge(&self, other: &Face) -> Option<Face> {
        if !self.coincident_planes(other) {
            return None;
        }

        None
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

fn point_inside_face(point: Vec2, face: Vec<Vec2>) -> bool {
    let n = face.len();
    let mut num_intersections = 0;

    for i in 0..n {
        let p1 = face[i];
        let p2 = face[(i + 1) % n];

        if point == p1 || point == p2 || point_on_edge(point, p1, p2)  {
            return true;
        }

        if (p1.y > point.y) != (p2.y > point.y) &&
           point.x < ((p2.x - p1.x) * (point.y - p1.y) / (p2.y - p1.y) + p1.x) {
            num_intersections += 1;
        }
    }

    num_intersections % 2 == 1
}

fn point_on_edge(point: Vec2, edge_start: Vec2, edge_end: Vec2) -> bool {
    // Check if the point is collinear with the edge and lies within the edge bounds
    (point.x - edge_start.x).abs() < f32::EPSILON
    && (point.y - edge_start.y).abs() < f32::EPSILON
    || (point.x - edge_end.x).abs() < f32::EPSILON
        && (point.y - edge_end.y).abs() < f32::EPSILON
    || ((point.x - edge_start.x) / (edge_end.x - edge_start.x)
            - (point.y - edge_start.y) / (edge_end.y - edge_start.y))
        .abs()
        < f32::EPSILON
        && point.x >= f32::min(edge_start.x, edge_end.x)
        && point.x <= f32::max(edge_start.x, edge_end.x)
        && point.y >= f32::min(edge_start.y, edge_end.y)
        && point.y <= f32::max(edge_start.y, edge_end.y)
}

pub fn merge_faces(faces: Vec<Face>) -> Vec<Face> {
    let mut final_faces = vec![];

    for i in 0..faces.len() {

        for j in 0..faces.len() {
            if i == j {
                continue;
            }

            let merged = faces[i].merge(&faces[j]);
        }

    }

    final_faces
}

pub fn default_wedge(size: Vec3) -> Vec<Face> {
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
}

pub fn ramp_inner_corner(size: Vec3) -> Vec<Face> {
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

pub fn ramp_crest(size: Vec3) -> Vec<Face> {
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
}

pub fn ramp_corner(size: Vec3) -> Vec<Face> {
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
}

pub fn microwedge_inner_corner(size: Vec3) -> Vec<Face> {
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
}

pub fn microwedge_corner(size: Vec3) -> Vec<Face> {
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

pub fn microwedge_half_outer_corner(size: Vec3) -> Vec<Face> {
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

pub fn microwedge_half_inner_corner_inverted(size: Vec3) -> Vec<Face> {
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
}

pub fn microwedge_half_inner_corner(size: Vec3) -> Vec<Face> {
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
}

pub fn microwedge_outer_corner(size: Vec3) -> Vec<Face> {
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
}

pub fn microwedge_triangle_corner(size: Vec3) -> Vec<Face> {
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
}

pub fn ramp(size: Vec3) -> Vec<Face> {
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
}

pub fn side_wedge(size: Vec3) -> Vec<Face> {
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
}

pub fn standard_brick(size: Vec3) -> Vec<Face> {
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
