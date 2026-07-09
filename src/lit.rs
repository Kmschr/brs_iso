use bevy::{prelude::*, light::{cluster::GlobalClusterSettings, DirectionalLightShadowMap, CascadeShadowConfig, CascadeShadowConfigBuilder}};

use crate::{bvh::BVHNode, cam::IsoCamera, state::{GameState, InputState}, SaveBVH};

const SHADOW_MAP_SIZE: usize = 8192;
const SUN_ILLUMINANCE: f32 = 20000.0;
// PCSS blocker-search radius in world units; tune for softer/harder shadow edges.
const SUN_SOFT_SHADOW_SIZE: f32 = 20.0;
// Depth range used before a save is loaded and cascade fitting kicks in.
const DEFAULT_SHADOW_DISTANCE: f32 = 4000000.0;

pub struct LightPlugin;

#[derive(Component)]
pub struct Sun;

impl Plugin for LightPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(DirectionalLightShadowMap { size: SHADOW_MAP_SIZE })
            // AmbientLight is a component in Bevy 0.19; it's placed on the camera.
            .add_systems(Startup, (spawn_light, raise_cluster_capacity))
            .add_systems(Update, (animate_light_direction, fit_shadow_cascades));
    }
}

// Saves can spawn hundreds of point/spot lights, which overflows the GPU
// clustering lists sized for typical scenes; Bevy grows them on demand but
// warns and may corrupt lighting for a few frames each time. Preallocate
// enough headroom up front (must run before the first frame renders, since
// per-view clustering buffers copy these initial capacities).
fn raise_cluster_capacity(mut settings: ResMut<GlobalClusterSettings>) {
    if let Some(gpu) = settings.gpu_clustering.as_mut() {
        gpu.initial_z_slice_list_capacity = gpu.initial_z_slice_list_capacity.max(8192);
        gpu.initial_index_list_capacity = gpu.initial_index_list_capacity.max(8192);
    }
}

fn spawn_light(mut commands: Commands) {

    let mut shadow_light_transform = Transform::from_rotation(Quat::from_rotation_x(-1.079));
    shadow_light_transform.rotate_y(0.303);
    shadow_light_transform.rotate_z(0.508);

    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            illuminance: SUN_ILLUMINANCE,
            soft_shadow_size: Some(SUN_SOFT_SHADOW_SIZE),
            ..default()
        },
        // A single cascade: with an orthographic camera the view frustum has the
        // same cross-section at every depth, so extra cascades add shadow passes
        // without adding any texel density. The depth range is refit to the
        // loaded scene every frame by fit_shadow_cascades.
        CascadeShadowConfig::from(CascadeShadowConfigBuilder {
            num_cascades: 1,
            minimum_distance: 0.1,
            maximum_distance: DEFAULT_SHADOW_DISTANCE,
            ..default()
        }),
        shadow_light_transform,
        Sun,
    ));

    // No second directional fill light: a shadowless fill washes out the sun's
    // cast shadows (Bevy's physical lighting makes it far stronger than the old
    // pre-0.14 model did). Ambient light on the camera provides gentle fill instead.
}

// Fit the shadow cascade's depth range to the loaded scene. The camera orbits at
// a distance proportional to the scene size, so a static range either clips
// casters or wastes depth precision on empty air.
fn fit_shadow_cascades(
    mut sun_query: Query<&mut CascadeShadowConfig, With<Sun>>,
    cam_query: Query<&Transform, With<IsoCamera>>,
    bvh_query: Query<&SaveBVH>,
) {
    let Ok(cam_transform) = cam_query.single() else { return; };
    let Ok(mut config) = sun_query.single_mut() else { return; };

    let forward = cam_transform.forward().as_vec3();
    let mut min_depth = f32::MAX;
    let mut max_depth = f32::MIN;

    for save_bvh in bvh_query.iter() {
        let BVHNode::Internal { aabb, .. } = &save_bvh.bvh[0] else { continue; };
        let center = aabb.center.as_vec3();
        let halfwidths = aabb.halfwidths.as_vec3();

        for i in 0..8 {
            let sign = Vec3::new(
                if i & 1 == 0 { -1.0 } else { 1.0 },
                if i & 2 == 0 { -1.0 } else { 1.0 },
                if i & 4 == 0 { -1.0 } else { 1.0 },
            );
            let corner = center + halfwidths * sign;
            let depth = (corner - cam_transform.translation).dot(forward);
            min_depth = min_depth.min(depth);
            max_depth = max_depth.max(depth);
        }
    }

    if min_depth >= max_depth {
        return;
    }

    let margin = (max_depth - min_depth) * 0.05;
    let new_min = (min_depth - margin).max(0.1);
    let new_max = max_depth + margin;

    let tolerance = (new_max - new_min) * 0.01;
    let unchanged = config.bounds.len() == 1
        && (config.minimum_distance - new_min).abs() < tolerance
        && (config.bounds[0] - new_max).abs() < tolerance;
    if unchanged {
        return;
    }

    config.minimum_distance = new_min;
    config.bounds = vec![new_max];
}

fn animate_light_direction(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<DirectionalLight>>,
    keycode: Res<ButtonInput<KeyCode>>,
    game_state: Res<GameState>,
) {
    match game_state.input {
        InputState::Listen => {},
        InputState::Typing => {
            return;
        }
    }

    let mut dir = 0.0;
    if keycode.pressed(KeyCode::ArrowLeft) {
        dir = 1.;
    } else if keycode.pressed(KeyCode::ArrowRight) {
        dir = -1.;
    }

    if dir == 0.0 {
        return;
    }

    for mut transform in &mut query {
        transform.rotate_y(time.delta_secs() * dir);
    }
}
