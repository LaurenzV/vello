use vello_common::paint::PremulColor;
use crate::fine2::FineKernel;

pub(crate) struct U8Kernel;

impl FineKernel for U8Kernel {
    type Numeric = u8;

    #[inline(always)]
    fn extract_color(color: PremulColor) -> [Self::Numeric; 4] {
        color.as_premul_rgba8().to_u8_array()
    }
}
