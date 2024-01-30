use bevy::prelude::*;

#[derive(Resource, Debug, Default)]
pub struct SceneAssets {
    pub diffuse_map: Handle<Image>,
    pub specular_map: Handle<Image>,
    pub plastic_material: Handle<StandardMaterial>,
    pub glow_material: Handle<StandardMaterial>,
    pub glass_material: Handle<StandardMaterial>,
    pub metal_material: Handle<StandardMaterial>
}

pub struct AssetLoaderPlugin;

impl Plugin for AssetLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SceneAssets>().add_systems(Startup, load_assets);
    }
}

fn load_assets(mut scene_assets: ResMut<SceneAssets>, asset_server: Res<AssetServer>, 
               mut materials: ResMut<Assets<StandardMaterial>>) {
    *scene_assets = SceneAssets {
        diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
        specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
        plastic_material: materials.add(StandardMaterial {
            base_color: Color::rgb(0.7, 0.7, 0.7),
            perceptual_roughness: 0.8,
            ..default()
        }),
        glow_material: materials.add(StandardMaterial {
            base_color: Color::rgb(1.0, 1.0, 1.0),
            perceptual_roughness: 0.8,
            ..default()
        }),
        glass_material: materials.add(StandardMaterial {
            base_color: Color::rgba(0.7, 0.7, 0.7, 0.9),
            perceptual_roughness: 0.8,
            alpha_mode: AlphaMode::Premultiplied,
            ..default()
        }),
        metal_material: materials.add(StandardMaterial {
            base_color: Color::rgb(0.7, 0.7, 0.7),
            perceptual_roughness: 0.8,
            metallic: 1.0,
            ..default()
        }),
    }
}
