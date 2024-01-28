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
}