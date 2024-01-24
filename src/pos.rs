use bevy::prelude::Vec2;
use brickadia::save::{Brick, Size};

pub fn _centroid(bricks: &[Brick]) -> Vec2 {
    if bricks.len() == 0 {
        return Vec2::ZERO;
    } else if bricks.len() == 1 {
        return Vec2::new(bricks[0].position.0 as f32, bricks[0].position.1 as f32);
    }

    // Sums for calculating Centroid of save
    let mut area_sum: i32 = 0;
    let mut point_sum = (0, 0);

    for brick in bricks {
        let size = _sizer(brick);

        // Add to Centroid calculation sums
        let area = size.0 * size.1;
        point_sum.0 += brick.position.0 * area as i32;
        point_sum.1 += brick.position.1 * area as i32;
        area_sum += area as i32;
    }

    // Calculate Centroid
    Vec2::new(
        point_sum.0 as f32 / area_sum as f32,
        point_sum.1 as f32 / area_sum as f32,
    )
}

fn _sizer(brick: &Brick) -> (u32, u32, u32) {
    match brick.size {
        Size::Empty => (0, 0, 0),
        Size::Procedural(x, y, z) => (x, y, z),
    }
}
