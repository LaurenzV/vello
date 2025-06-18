use vello_common::paint::PremulColor;
use vello_common::tile::Tile;
use crate::fine2::FineKernel;
use crate::fine::COLOR_COMPONENTS;
use crate::region::Region;

pub(crate) struct F32Kernel;

impl FineKernel for F32Kernel {
    type Numeric = f32;

    fn extract_color(color: PremulColor) -> [Self::Numeric; 4] {
        color.as_premul_f32().components
    }

    fn pack(region: &mut Region<'_>, blend_buf: &[Self::Numeric]) {
        for y in 0..Tile::HEIGHT {
            for (x, pixel) in region
                .row_mut(y)
                .chunks_exact_mut(COLOR_COMPONENTS)
                .enumerate()
            {
                let idx = COLOR_COMPONENTS * (usize::from(Tile::HEIGHT) * x + usize::from(y));
                let start = &blend_buf[idx..];
                // TODO: Use SIMD
                let converted = [
                    (start[0] * 255.0 + 0.5) as u8,
                    (start[1] * 255.0 + 0.5) as u8,
                    (start[2] * 255.0 + 0.5) as u8,
                    (start[3] * 255.0 + 0.5) as u8,
                ];
                pixel.copy_from_slice(&converted);
            }
        }
    }
}

mod fill {
    use vello_common::fearless_simd::{u32x4, u32x8, u8x32, Simd, SimdBase};
    use crate::fine2::TILE_HEIGHT_COMPONENTS;

    fn alpha_composite<S: Simd>(
        s: S,
        target: &mut [u8],
        src_c: [u8; 4],
    ) {
        let src_c = u32x8::block_splat(u32x4::splat(s, u32::from_ne_bytes(src_c))).reinterpret_u8();
        
        for target_p in target.chunks_exact_mut(32) {
            let target_p = u8x32::from_slice(s, target_p);
            for bg_c in strip.chunks_exact_mut(COLOR_COMPONENTS) {
                let src_c = source.next().unwrap();
                for i in 0..COLOR_COMPONENTS {
                    bg_c[i] = src_c[i].add(bg_c[i].normalized_mul(src_c[3].one_minus()));
                }
            }
        }
    }
}