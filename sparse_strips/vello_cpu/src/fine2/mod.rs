mod highp;
mod lowp;

use core::fmt::Debug;
use vello_common::coarse::WideTile;
use vello_common::paint::PremulColor;
use vello_common::tile::Tile;
use crate::region::Region;

pub(crate) const COLOR_COMPONENTS: usize = 4;
pub(crate) const TILE_HEIGHT_COMPONENTS: usize = Tile::HEIGHT as usize * COLOR_COMPONENTS;
#[doc(hidden)]
pub const SCRATCH_BUF_SIZE: usize =
    WideTile::WIDTH as usize * Tile::HEIGHT as usize * COLOR_COMPONENTS;

pub type ScratchBuf<F> = [F; SCRATCH_BUF_SIZE];

pub trait Numeric: Copy + Default + Clone + Debug + PartialEq {
    const ZERO: Self;
}

impl Numeric for f32 {
    const ZERO: Self = 0.0;
}

impl Numeric for u8 {
    const ZERO: Self = 0;
}

pub trait FineKernel {
    type Numeric: Numeric;
    
    fn extract_color(color: PremulColor) -> [Self::Numeric; 4];
    fn pack(region: &mut Region<'_>, blend_buf: &[Self::Numeric]);
}


#[derive(Debug)]
pub struct Fine<T: FineKernel> {
    pub(crate) wide_coords: (u16, u16),
    pub(crate) blend_buf: Vec<ScratchBuf<T::Numeric>>,
    pub(crate) paint_buf: ScratchBuf<T::Numeric>,
}

impl<T: FineKernel> Fine<T> {
    pub fn new() -> Self {
        Self {
            wide_coords: (0, 0),
            blend_buf: vec![[T::Numeric::ZERO; SCRATCH_BUF_SIZE]],
            paint_buf: [T::Numeric::ZERO; SCRATCH_BUF_SIZE],
        }
    }

    pub fn set_coords(&mut self, x: u16, y: u16) {
        self.wide_coords = (x, y);
    }

    pub fn clear(&mut self, premul_color: PremulColor) {
        let converted_color = T::extract_color(premul_color);
        let blend_buf = self.blend_buf.last_mut().unwrap();

        if converted_color[0] == converted_color[1]
            && converted_color[1] == converted_color[2]
            && converted_color[2] == converted_color[3]
        {
            // All components are the same, so we can use memset instead.
            blend_buf.fill(converted_color[0]);
        } else {
            // TODO: Faster with 512x SIMD?
            for z in blend_buf.chunks_exact_mut(COLOR_COMPONENTS) {
                z.copy_from_slice(&converted_color);
            }
        }
    }

    pub fn pack(&self, region: &mut Region<'_>) {
        let blend_buf = self.blend_buf.last().unwrap();
        
        T::pack(region, blend_buf);
    }
}