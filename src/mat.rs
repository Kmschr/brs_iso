
#[derive(Debug, Clone, Copy)]
pub enum BrickMaterial {
    Plastic,
    Glow,
    Glass,
    Metallic,
    Unknown
}

impl From<&str> for BrickMaterial {
    fn from(s: &str) -> Self {
        match s {
            "BMC_Plastic" => BrickMaterial::Plastic,
            "BMC_Glow" => BrickMaterial::Glow,
            "BMC_Glass" => BrickMaterial::Glass,
            "BMC_Metallic" => BrickMaterial::Metallic,
            _ => BrickMaterial::Unknown
        }
    }
}
