use bevy::prelude::*;

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
    pub color_override: bool,
}

impl Face {
    pub fn new(verts: Vec<Vec3>) -> Self {
        Face {
            verts,
            color_override: false,
            ..default()
        }
    }

    pub fn calc_normal(&mut self) {
        assert!(self.verts.len() >= 3);
        let a = self.verts[0];
        let b = self.verts[1];
        let c = self.verts[2];

        let dir = (b - a).cross(c - a);
        let mut normal = -dir.normalize();

        if normal.z < 0.000001 {
            normal.z = 0.;
        }

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

    fn to_2d(&self) -> Vec<Vec2> {
        let mut points = Vec::with_capacity(self.verts.len());

        for p in &self.verts {
            let n = &self.normal;

            // Select a non-zero vector V not parallel to N
            let v = if n.z == 0. {
                Vec3::new(0., n.z, -n.y)
            } else {
                Vec3::new(n.y, -n.x, 0.)
            };

            // Calculate the cross product U = N X V
            let u = n.cross(v);

            // Normalize V and U
            let u = u.normalize();
            let v = v.normalize();

            let x = u.dot(*p);
            let y = v.dot(*p);

            points.push(Vec2::new(x, y));
        }

        vec![]
    }

    pub fn inside(&self, other: &Face) -> bool {
        if self.normal != other.normal {
            info!("normals were not equal??? - {} != {}", self.normal, other.normal);
            return false;
        }

        for vert in self.to_2d() {
            if !point_inside_face(vert, other.to_2d()) {
                return false;
            }
        }

        true
    }
}

fn point_inside_face(point: Vec2, face: Vec<Vec2>) -> bool {
    let n = face.len();
    let mut num_intersections = 0;

    for i in 0..n {
        let p1 = face[i];
        let p2 = face[(i + 1) % n];

        if (p1.y > point.y) != (p2.y > point.y) &&
           point.x < ((p2.x - p1.x) * (point.y - p1.y) / (p2.y - p1.y) + p1.x) {
            num_intersections += 1;
        }
    }

    num_intersections % 2 == 1
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
