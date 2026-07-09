use brickadia::save::Color;

pub fn cc(c: &Color) -> [f32; 4] {
    [
        c.r as f32 / 255.0,
        c.g as f32 / 255.0,
        c.b as f32 / 255.0,
        0.0,
    ]
}

// Same color as `cc` but as raw bytes for the packed Unorm8x4 vertex format;
// the GPU divides by 255 during the vertex fetch.
pub fn cu8(c: &Color) -> [u8; 4] {
    [c.r, c.g, c.b, 0]
}
