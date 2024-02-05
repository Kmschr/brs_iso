use bevy::math::{IVec3, Ray};
use brickadia::{save::{SaveData, Brick}, util::get_axis_size};

#[derive(Debug, Default, Clone, Copy)]
pub struct AABB {
    pub center: IVec3,
    pub halfwidths: IVec3,
}

impl AABB {
    pub fn from_brick(brick: &Brick, save_data: &SaveData) -> Self {
        let pos = IVec3::new(
            brick.position.0 as i32,
            brick.position.2 as i32,
            brick.position.1 as i32,
        );
        let w = get_axis_size(brick, &save_data.header2.brick_assets, 0) as i32;
        let h = get_axis_size(brick, &save_data.header2.brick_assets, 2) as i32;
        let l = get_axis_size(brick, &save_data.header2.brick_assets, 1) as i32;
        Self {
            center: pos,
            halfwidths: IVec3::new(w, h, l),
        }
    }

    pub fn neighbors(&self, other: &AABB) -> bool {
        if (self.center.x - other.center.x).abs() > (self.halfwidths.x + other.halfwidths.x) {
            return false;
        }
        if (self.center.y - other.center.y).abs() > (self.halfwidths.y + other.halfwidths.y) {
            return false;
        }
        if (self.center.z - other.center.z).abs() > (self.halfwidths.z + other.halfwidths.z) {
            return false;
        }
        true
    }

    pub fn intersects(&self, ray: Ray) -> bool {
        let center = self.center.as_vec3();
        let halfwidths = self.halfwidths.as_vec3();

        let t1 = (center.x - halfwidths.x - ray.origin.x) / ray.direction.x;
        let t2 = (center.x + halfwidths.x - ray.origin.x) / ray.direction.x;
        let t3 = (center.y - halfwidths.y - ray.origin.y) / ray.direction.y;
        let t4 = (center.y + halfwidths.y - ray.origin.y) / ray.direction.y;
        let t5 = (center.z - halfwidths.z - ray.origin.z) / ray.direction.z;
        let t6 = (center.z + halfwidths.z - ray.origin.z) / ray.direction.z;

        let tmin = t1.min(t2).max(t3.min(t4).max(t5.min(t6)));
        let tmax = t1.max(t2).min(t3.max(t4).min(t5.max(t6)));

        tmax >= 0.0 && tmin <= tmax
    }

    pub fn volume(&self) -> i64 {
        let size = self.halfwidths * 2;
        size.x as i64 * size.y as i64 * size.z as i64
    }
}