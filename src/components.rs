use bevy::prelude::*;
use brickadia::save::{UnrealType, SaveData, BrickColor};

use crate::utils::cc;

const BRIGHTNESS_MULTIPLIER: f32 = 20000.0;

pub fn gen_point_lights(save_data: &SaveData) -> Vec<PointLightBundle> {
    if !save_data.components.contains_key("BCD_PointLight") {
        return vec![];
    }

    let mut point_lights = Vec::with_capacity(save_data.components["BCD_PointLight"].brick_indices.len());

    let color_palette = &save_data
        .header2
        .colors
        .iter()
        .map(|color| cc(&color))
        .collect::<Vec<[f32; 4]>>();

    for i in &save_data.components["BCD_PointLight"].brick_indices {
        let brick = &save_data.bricks[*i as usize];
        let component = &brick.components["BCD_PointLight"];

        let use_brick_color = as_bool(&component["bUseBrickColor"]);

        let color = if use_brick_color {
            match &brick.color {
                BrickColor::Index(i) => color_palette[*i as usize],
                BrickColor::Unique(color) => cc(color),
            }
        } else {
            let color = as_color(&component["Color"]);
            cc(&color)
        };

        let radius = as_f32(&component["Radius"]);
        let _shadows = as_bool(&component["bCastShadows"]);
        let brightness = as_f32(&component["Brightness"]);

        let translation = Vec3::new(
            brick.position.0 as f32,
            brick.position.2 as f32,
            brick.position.1 as f32,
        );

        point_lights.push(PointLightBundle {
            point_light: PointLight {
                color: Color::rgb(color[0], color[1], color[2]),
                radius,
                intensity: brightness * BRIGHTNESS_MULTIPLIER,
                range: radius * 2.0,
                shadows_enabled: false,
                ..default()
            },
            transform: Transform::from_translation(translation),
            ..default()
        });
    }

    point_lights
}

pub fn gen_spot_lights(save_data: &SaveData) -> Vec<SpotLightBundle> {
    if !save_data.components.contains_key("BCD_SpotLight") {
        return vec![];
    }

    let mut spot_lights = Vec::with_capacity(save_data.components["BCD_SpotLight"].brick_indices.len());

    let color_palette = &save_data
        .header2
        .colors
        .iter()
        .map(|color| cc(&color))
        .collect::<Vec<[f32; 4]>>();

    for i in &save_data.components["BCD_SpotLight"].brick_indices {
        let brick = &save_data.bricks[*i as usize];
        let component = &brick.components["BCD_SpotLight"];

        let use_brick_color = as_bool(&component["bUseBrickColor"]);

        let color = if use_brick_color {
            match &brick.color {
                BrickColor::Index(i) => color_palette[*i as usize],
                BrickColor::Unique(color) => cc(color),
            }
        } else {
            let color = as_color(&component["Color"]);
            cc(&color)
        };

        let radius = as_f32(&component["Radius"]);
        let _shadows = as_bool(&component["bCastShadows"]);
        let brightness = as_f32(&component["Brightness"]);
        let inner_angle = as_f32(&component["InnerConeAngle"]);
        let outer_angle = as_f32(&component["OuterConeAngle"]);
        let rotation = as_rotation(&component["Rotation"]);

        let translation = Vec3::new(
            brick.position.0 as f32,
            brick.position.2 as f32,
            brick.position.1 as f32,
        );

        let mut transform = Transform::from_translation(translation);
        transform.rotate_axis(Vec3::Y, (-90f32).to_radians());

        transform.rotate_axis(Vec3::Z, rotation.x.to_radians());
        transform.rotate_axis(Vec3::NEG_Y, rotation.y.to_radians());

        spot_lights.push(SpotLightBundle {
            spot_light: SpotLight {
                color: Color::rgb(color[0], color[1], color[2]),
                radius,
                intensity: brightness * BRIGHTNESS_MULTIPLIER,
                range: radius * 2.0,
                shadows_enabled: false,
                inner_angle: inner_angle.to_radians(),
                outer_angle: outer_angle.to_radians(),
                ..default()
            },
            transform,
            ..default()
        });
    }

    spot_lights
}

fn as_bool(b: &UnrealType) -> bool {
    match b {
        UnrealType::Boolean(val) => *val,
        _ => unreachable!()
    }
}

fn as_color(c: &UnrealType) -> &brickadia::save::Color {
    match c {
        UnrealType::Color(c) => c,
        _ => unreachable!()
    }
}

fn as_f32(n: &UnrealType) -> f32 {
    match n {
        UnrealType::Float(val) => *val,
        _ => unreachable!()
    }
}

fn as_rotation(r: &UnrealType) -> Vec3 {
    match r {
        UnrealType::Rotator(roll, pitch, yaw) => Vec3::new(*roll, *pitch, *yaw),
        _ => unreachable!()
    }
}
