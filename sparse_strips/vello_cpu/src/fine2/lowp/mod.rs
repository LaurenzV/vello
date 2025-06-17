use vello_common::paint::PremulColor;
use vello_common::tile::Tile;
use crate::fine2::FineKernel;
use crate::fine::COLOR_COMPONENTS;
use crate::region::Region;

pub(crate) struct U8Kernel;

impl FineKernel for U8Kernel {
    type Numeric = u8;

    fn extract_color(color: PremulColor) -> [Self::Numeric; 4] {
        color.as_premul_rgba8().to_u8_array()
    }

    fn pack(region: &mut Region<'_>, blend_buf: &[Self::Numeric]) {
        for y in 0..Tile::HEIGHT {
            for (x, pixel) in region
                .row_mut(y)
                .chunks_exact_mut(COLOR_COMPONENTS)
                .enumerate()
            {
                let idx = COLOR_COMPONENTS * (usize::from(Tile::HEIGHT) * x + usize::from(y));
                pixel.copy_from_slice(&blend_buf[idx..][..COLOR_COMPONENTS]);
            }
        }
    }
}
