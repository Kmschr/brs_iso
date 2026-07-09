use bevy::{
    asset::RenderAssetUsages,
    camera::{visibility::RenderLayers, ClearColorConfig, ScalingMode, Viewport},
    material::OpaqueRendererMethod,
    mesh::Indices,
    prelude::*,
    render::render_resource::PrimitiveTopology,
    window::PrimaryWindow,
};

use crate::cam::IsoCamera;
use crate::state::BuildLoaded;

// CAD-style view cube: a chamfered cube rendered by a second camera into a
// small corner viewport. Faces, edge bevels, and corners are separate meshes;
// hovering highlights the region and clicking snaps the camera to that view.

const CUBE_LAYER: usize = 1;
// Chamfer width; faces span [-FACE_EXTENT, FACE_EXTENT] on a unit half-size cube.
const CHAMFER: f32 = 0.3;
const FACE_EXTENT: f32 = 1.0 - CHAMFER;
// Logical pixels of the corner viewport, top-right below the FPS counter.
const VIEWPORT_SIZE: f32 = 130.0;
const VIEWPORT_MARGIN: f32 = 14.0;
const VIEWPORT_TOP: f32 = 52.0;
// Fits the cube's rotated diagonal (2√3) with a little margin.
const ORTHO_HEIGHT: f32 = 3.9;

const FACE_COLOR: Color = Color::WHITE;
const BEVEL_COLOR: Color = Color::srgb(0.71, 0.72, 0.76);
const HOVER_COLOR: Color = Color::srgb(1.0, 0.62, 0.15);

pub struct ViewCubePlugin;

// True while the cursor is over the cube's viewport, so other systems (brick
// hover info) can yield to it.
#[derive(Resource, Default)]
pub struct ViewCubeHover(pub bool);

#[derive(Component)]
struct ViewCubeCam;

// One clickable region of the cube. `dir` holds -1/0/1 per axis: one nonzero
// component for faces, two for edge bevels, three for corners.
#[derive(Component)]
struct CubePiece {
    dir: IVec3,
    base: Color,
}

impl Plugin for ViewCubePlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<ViewCubeHover>()
            .add_systems(Startup, spawn_viewcube)
            .add_systems(Update, (toggle_viewcube, fit_viewcube_viewport, sync_viewcube_cam, pick_viewcube));
    }
}

fn spawn_viewcube(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn((
        Camera3d::default(),
        Camera {
            // Render after (on top of) the main camera, into a corner viewport
            // that fit_viewcube_viewport keeps sized to the window.
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical { viewport_height: ORTHO_HEIGHT },
            ..OrthographicProjection::default_3d()
        }),
        Msaa::Off,
        RenderLayers::layer(CUBE_LAYER),
        Transform::from_xyz(0.0, 0.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        ViewCubeCam,
    ));

    // Label orientation: t_u is the world direction of increasing texture u,
    // t_v of increasing texture v (down-screen), chosen so labels read upright
    // in each face's own axis view.
    let s = FACE_EXTENT;
    let faces: [(IVec3, Vec3, Vec3, &str); 6] = [
        (IVec3::Z, Vec3::X, Vec3::NEG_Y, "front"),
        (IVec3::X, Vec3::NEG_Z, Vec3::NEG_Y, "right"),
        (IVec3::NEG_Z, Vec3::NEG_X, Vec3::NEG_Y, "back"),
        (IVec3::NEG_X, Vec3::Z, Vec3::NEG_Y, "left"),
        (IVec3::Y, Vec3::X, Vec3::Z, "top"),
        (IVec3::NEG_Y, Vec3::X, Vec3::NEG_Z, "bottom"),
    ];
    for (dir, t_u, t_v, label) in faces {
        let normal = dir.as_vec3();
        let corner_uvs = [Vec2::new(0., 0.), Vec2::new(1., 0.), Vec2::new(1., 1.), Vec2::new(0., 1.)];
        let verts = corner_uvs
            .map(|uv| normal + t_u * s * (2.0 * uv.x - 1.0) + t_v * s * (2.0 * uv.y - 1.0));
        let mesh = piece_mesh(&verts, Some(&corner_uvs), normal);
        let material = materials.add(StandardMaterial {
            base_color: FACE_COLOR,
            base_color_texture: Some(asset_server.load(format!("embedded://viewcube/{label}.png"))),
            ..piece_material()
        });
        spawn_piece(&mut commands, &mut meshes, mesh, material, dir, FACE_COLOR);
    }

    // 12 edge bevels: a strip between each pair of adjacent faces.
    for i in 0..3usize {
        for j in i + 1..3 {
            let k = 3 - i - j;
            for si in [-1, 1] {
                for sj in [-1, 1] {
                    let ea = axis(i) * si as f32;
                    let eb = axis(j) * sj as f32;
                    let ef = axis(k);
                    let verts = [
                        ea + eb * s + ef * s,
                        ea + eb * s - ef * s,
                        ea * s + eb - ef * s,
                        ea * s + eb + ef * s,
                    ];
                    let dir = (ea + eb).as_ivec3();
                    let mesh = piece_mesh(&verts, None, ea + eb);
                    let material = materials.add(StandardMaterial {
                        base_color: BEVEL_COLOR,
                        ..piece_material()
                    });
                    spawn_piece(&mut commands, &mut meshes, mesh, material, dir, BEVEL_COLOR);
                }
            }
        }
    }

    // 8 corner triangles closing the chamfer.
    for sx in [-1, 1] {
        for sy in [-1, 1] {
            for sz in [-1, 1] {
                let dir = IVec3::new(sx, sy, sz);
                let d = dir.as_vec3();
                let verts = [
                    Vec3::new(d.x, d.y * s, d.z * s),
                    Vec3::new(d.x * s, d.y, d.z * s),
                    Vec3::new(d.x * s, d.y * s, d.z),
                ];
                let mesh = piece_mesh(&verts, None, d);
                let material = materials.add(StandardMaterial {
                    base_color: BEVEL_COLOR,
                    ..piece_material()
                });
                spawn_piece(&mut commands, &mut meshes, mesh, material, dir, BEVEL_COLOR);
            }
        }
    }
}

fn piece_material() -> StandardMaterial {
    StandardMaterial {
        unlit: true,
        // The global default is deferred, but the cube camera has no deferred
        // prepasses; force these through the forward path.
        opaque_render_method: OpaqueRendererMethod::Forward,
        ..default()
    }
}

fn axis(i: usize) -> Vec3 {
    match i {
        0 => Vec3::X,
        1 => Vec3::Y,
        _ => Vec3::Z,
    }
}

fn spawn_piece(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mesh: Mesh,
    material: Handle<StandardMaterial>,
    dir: IVec3,
    base: Color,
) {
    commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(material),
        RenderLayers::layer(CUBE_LAYER),
        CubePiece { dir, base },
    ));
}

// Triangle or quad with a flat normal, wound to face `outward`.
fn piece_mesh(verts: &[Vec3], uvs: Option<&[Vec2; 4]>, outward: Vec3) -> Mesh {
    let mut indices: Vec<u32> = match verts.len() {
        3 => vec![0, 1, 2],
        _ => vec![0, 1, 2, 0, 2, 3],
    };
    if (verts[1] - verts[0]).cross(verts[2] - verts[0]).dot(outward) < 0.0 {
        indices.reverse();
    }

    let normal = outward.normalize().to_array();
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, verts.iter().map(|v| v.to_array()).collect::<Vec<_>>());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![normal; verts.len()]);
    if let Some(uvs) = uvs {
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs.iter().map(|uv| uv.to_array()).collect::<Vec<_>>());
    }
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

// Hide the cube (and stop it eating clicks) until a build is loaded.
fn toggle_viewcube(
    build_loaded: Res<BuildLoaded>,
    mut cam_query: Query<&mut Camera, With<ViewCubeCam>>,
) {
    let Ok(mut camera) = cam_query.single_mut() else { return; };
    if camera.is_active != build_loaded.0 {
        camera.is_active = build_loaded.0;
    }
}

// Keep the cube viewport pinned to the window's bottom-right corner.
fn fit_viewcube_viewport(
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut cam_query: Query<&mut Camera, With<ViewCubeCam>>,
) {
    let Ok(window) = window_query.single() else { return; };
    let Ok(mut camera) = cam_query.single_mut() else { return; };

    let scale = window.scale_factor();
    let size = ((VIEWPORT_SIZE * scale) as u32)
        .min(window.physical_width())
        .min(window.physical_height())
        .max(1);
    let margin = (VIEWPORT_MARGIN * scale) as u32;
    let top = ((VIEWPORT_TOP * scale) as u32).min(window.physical_height().saturating_sub(size));
    let position = UVec2::new(
        window.physical_width().saturating_sub(size + margin),
        top,
    );

    let current = camera.viewport.as_ref().map(|v| (v.physical_position, v.physical_size));
    if current != Some((position, UVec2::splat(size))) {
        camera.viewport = Some(Viewport {
            physical_position: position,
            physical_size: UVec2::splat(size),
            ..default()
        });
    }
}

// The cube itself stays fixed in its own little world; its camera copies the
// main camera's orientation so the cube always mirrors the current view.
fn sync_viewcube_cam(
    main_query: Query<&Transform, (With<IsoCamera>, Without<ViewCubeCam>)>,
    mut cube_query: Query<&mut Transform, With<ViewCubeCam>>,
) {
    let Ok(main_transform) = main_query.single() else { return; };
    let Ok(mut cube_transform) = cube_query.single_mut() else { return; };
    cube_transform.rotation = main_transform.rotation;
    cube_transform.translation = main_transform.rotation * Vec3::new(0.0, 0.0, 10.0);
}

fn pick_viewcube(
    window_query: Query<&Window, With<PrimaryWindow>>,
    cube_cam_query: Query<(&Camera, &GlobalTransform), With<ViewCubeCam>>,
    mut iso_cam_query: Query<&mut IsoCamera>,
    pieces: Query<(&CubePiece, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut hover_state: ResMut<ViewCubeHover>,
    build_loaded: Res<BuildLoaded>,
    mut hovered: Local<Option<IVec3>>,
) {
    if !build_loaded.0 {
        hover_state.0 = false;
        return;
    }

    let (over_viewport, region) = hovered_region(&window_query, &cube_cam_query);
    hover_state.0 = over_viewport;

    if region != *hovered {
        for (piece, material) in &pieces {
            if Some(piece.dir) != region && Some(piece.dir) != *hovered {
                continue;
            }
            if let Some(mut material) = materials.get_mut(&material.0) {
                material.base_color = if Some(piece.dir) == region { HOVER_COLOR } else { piece.base };
            }
        }
        *hovered = region;
    }

    if let Some(dir) = region {
        if mouse.just_pressed(MouseButton::Left) {
            if let Ok(mut cam) = iso_cam_query.single_mut() {
                if let Some((horizontal, vertical)) = region_angles(dir, cam.horizontal_angle) {
                    cam.horizontal_angle = horizontal;
                    cam.vertical_angle = vertical;
                }
            }
        }
    }
}

// Whether the cursor is over the cube viewport, and which cube region (if
// any) it points at.
fn hovered_region(
    window_query: &Query<&Window, With<PrimaryWindow>>,
    cube_cam_query: &Query<(&Camera, &GlobalTransform), With<ViewCubeCam>>,
) -> (bool, Option<IVec3>) {
    let inner = || {
        let window = window_query.single().ok()?;
        let cursor = window.cursor_position()?;
        let (camera, cam_transform) = cube_cam_query.single().ok()?;
        let viewport = camera.viewport.as_ref()?;

        let scale = window.scale_factor();
        let min = viewport.physical_position.as_vec2() / scale;
        let size = viewport.physical_size.as_vec2() / scale;
        let local = cursor - min;
        if local.x < 0.0 || local.y < 0.0 || local.x > size.x || local.y > size.y {
            return None;
        }

        // viewport_to_world subtracts the viewport offset itself, so it takes
        // window coordinates, not viewport-relative ones.
        let region = camera.viewport_to_world(cam_transform, cursor).ok()
            .and_then(|ray| ray_box_hit(ray.origin, *ray.direction))
            .map(classify);
        Some(region)
    };
    match inner() {
        Some(region) => (true, region),
        None => (false, None),
    }
}

// Slab test against the cube's bounding box [-1, 1]^3; returns the entry point.
fn ray_box_hit(origin: Vec3, dir: Vec3) -> Option<Vec3> {
    let inv = dir.recip();
    let t1 = (Vec3::NEG_ONE - origin) * inv;
    let t2 = (Vec3::ONE - origin) * inv;
    let tmin = t1.min(t2).max_element();
    let tmax = t1.max(t2).min_element();
    (tmax >= tmin && tmax >= 0.0).then(|| origin + dir * tmin.max(0.0))
}

// Map a surface point to a region: components beyond the face extent count as
// that axis's sign, matching how the chamfer geometry is laid out.
fn classify(point: Vec3) -> IVec3 {
    let component = |v: f32| {
        if v > FACE_EXTENT { 1 } else if v < -FACE_EXTENT { -1 } else { 0 }
    };
    IVec3::new(component(point.x), component(point.y), component(point.z))
}

// Region direction → camera angles. Horizontal angle comes from the region's
// XZ heading; regions on the top ring pitch to 45°, side regions to 90°.
// Bottom regions map to their side equivalent (the camera can't go below the
// horizon), except the bottom face itself, which has no valid view.
fn region_angles(dir: IVec3, current_horizontal: f32) -> Option<(f32, f32)> {
    if dir.x == 0 && dir.z == 0 {
        // Top-down view keeps the current heading.
        return (dir.y > 0).then_some((current_horizontal, 0.0));
    }
    let horizontal = (dir.z as f32).atan2(dir.x as f32).to_degrees().rem_euclid(360.0);
    let vertical = if dir.y > 0 { 45.0 } else { 90.0 };
    Some((horizontal, vertical))
}
