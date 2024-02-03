use bevy::math::IVec3;
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

    pub fn from_aabbs(aabbs: &[&AABB]) -> Self {
        let mut min = IVec3::new(i32::MAX, i32::MAX, i32::MAX);
        let mut max = IVec3::new(i32::MIN, i32::MIN, i32::MIN);
        for aabb in aabbs {
            min = min.min(aabb.center - aabb.halfwidths);
            max = max.max(aabb.center + aabb.halfwidths);
        }
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
        Self { center, halfwidths }
    }

    pub fn intersects(&self, other: &AABB) -> bool {
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

    pub fn volume(&self) -> i64 {
        let size = self.halfwidths * 2;
        size.x as i64 * size.y as i64 * size.z as i64
    }
}