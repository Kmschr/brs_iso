use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct SceneAssets {
    pub diffuse_map: Handle<Image>,
    pub specular_map: Handle<Image>,
    pub materials: Materials,
    pub sounds: Sounds,
}

#[derive(Default)]
pub struct Materials {
    pub plastic: Handle<StandardMaterial>,
    pub glow: Handle<StandardMaterial>,
    pub glass: Handle<StandardMaterial>,
    pub metal: Handle<StandardMaterial>,
    pub water: Handle<StandardMaterial>,
}

#[derive(Default)]
pub struct Sounds {
    pub startup: Handle<AudioSource>,
    pub clear_bricks: Handle<AudioSource>,
    pub upload_start: Handle<AudioSource>,
    pub upload_end: Handle<AudioSource>,
}

pub struct AssetLoaderPlugin;

impl Plugin for AssetLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SceneAssets>().add_systems(Startup, load_assets);
    }
}

fn load_assets(
    mut scene_assets: ResMut<SceneAssets>, 
    asset_server: Res<AssetServer>, 
    mut materials: ResMut<Assets<StandardMaterial>>) 
{
    *scene_assets = SceneAssets {
        diffuse_map: asset_server.load("embedded://environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
        specular_map: asset_server.load("embedded://environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
        materials: Materials {
            plastic: materials.add(StandardMaterial {
                base_color: Color::rgb(0.7, 0.7, 0.7),
                perceptual_roughness: 0.8,
                ..default()
            }),
            glow: materials.add(StandardMaterial {
                base_color: Color::rgb(1.0, 1.0, 1.0),
                perceptual_roughness: 0.8,
                ..default()
            }),
            glass: materials.add(StandardMaterial {
                base_color: Color::rgba(0.7, 0.7, 0.7, 0.9),
                perceptual_roughness: 0.8,
                alpha_mode: AlphaMode::Premultiplied,
                ..default()
            }),
            metal: materials.add(StandardMaterial {
                base_color: Color::rgb(0.7, 0.7, 0.7),
                perceptual_roughness: 0.3,
                metallic: 1.0,
                ..default()
            }),
            water: materials.add(StandardMaterial {
                base_color: Color::rgba(0.0, 0.2, 0.4, 0.6),
                alpha_mode: AlphaMode::Premultiplied,
                ior: 1.33,
                reflectance: 0.25,
                thickness: 2000.0,
                ..default()
            }),
        },
        sounds: Sounds {
            startup: asset_server.load("embedded://sounds/playerConnect.wav"),
            clear_bricks: asset_server.load("embedded://sounds/brickClear.wav"),
            upload_start: asset_server.load("embedded://sounds/uploadStart.wav"),
            upload_end: asset_server.load("embedded://sounds/uploadEnd.wav"),
        }
    }
}
