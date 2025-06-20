mod highp;
mod lowp;

use alloc::vec;
use alloc::vec::Vec;
use core::fmt::Debug;
use vello_common::coarse::{Cmd, WideTile};
use vello_common::encode::{EncodedKind, EncodedPaint};
use vello_common::paint::{Paint, PremulColor};
use vello_common::tile::Tile;
use crate::fine::{fill, FineType};
use crate::Level;
use crate::peniko::{BlendMode, Compose, Mix};
use crate::region::Region;

pub(crate) const COLOR_COMPONENTS: usize = 4;
pub(crate) const TILE_HEIGHT_COMPONENTS: usize = Tile::HEIGHT as usize * COLOR_COMPONENTS;
#[doc(hidden)]
pub const SCRATCH_BUF_SIZE: usize =
    WideTile::WIDTH as usize * Tile::HEIGHT as usize * COLOR_COMPONENTS;

pub use lowp::U8Kernel;
pub use highp::F32Kernel;
use vello_common::fearless_simd::{f32x16, f32x4, f32x8, Simd, SimdBase, SimdFloat, SimdInto};
use crate::fine2::highp::rounded_blurred_rect::BlurredRoundedRectFiller;

pub type ScratchBuf<F> = [F; SCRATCH_BUF_SIZE];


pub trait Numeric: Copy + Default + Clone + Debug + PartialEq + Send + Sync + 'static {
    const ZERO: Self;
    const ONE: Self;
}

impl Numeric for f32 {
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;
}

impl Numeric for u8 {
    const ZERO: Self = 0;
    const ONE: Self = 255;
}

pub trait FineKernel: Send + Sync + 'static {
    type Numeric: Numeric;
    
    fn extract_color(color: PremulColor) -> [Self::Numeric; 4];
    fn pack(region: &mut Region<'_>, blend_buf: &[Self::Numeric]);
    fn fill_buf_solid<S: Simd>(simd: S, target: &mut [Self::Numeric], color: [Self::Numeric; 4]);
    fn fill_buf_arbitrary<S: Simd>(
        simd: S,
        target: &mut [Self::Numeric],
        src: impl Iterator<Item = f32x16<S>>
    );
    fn fill_solid<S: Simd>(
        simd: S,
        target: &mut [Self::Numeric],
        color: [Self::Numeric; 4],
        blend_mode: BlendMode,
    );
    // fn fill_arbitrary<S: Simd>(
    //     simd: S,
    //     target: &mut [Self::Numeric],
    //     shader_src: impl Iterator<Item = f32x16<S>>,
    //     blend_mode: BlendMode,
    // );
    fn strip_solid<S: Simd>(
        simd: S,
        target: &mut [Self::Numeric],
        color: [Self::Numeric; 4],
        blend_mode: BlendMode,
        alphas: &[u8]
    );
    // fn strip_shader<S: Simd>(
    //     simd: S,
    //     target: &mut [Self::Numeric],
    //     shader_src: impl Iterator<Item = f32x16<S>>,
    //     blend_mode: BlendMode,
    //     alphas: &[u8]
    // );
}


#[derive(Debug)]
pub struct Fine<T: FineKernel, S: Simd> {
    pub(crate) wide_coords: (u16, u16),
    pub(crate) blend_buf: Vec<ScratchBuf<T::Numeric>>,
    pub(crate) paint_buf: ScratchBuf<T::Numeric>,
    pub(crate) simd: S,
}

impl<T: FineKernel, S: Simd> Fine<T, S> {
    pub fn new(simd: S) -> Self {
        Self {
            simd,
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

        T::fill_buf_solid(self.simd, blend_buf, converted_color);
    }

    pub fn pack(&self, region: &mut Region<'_>) {
        let blend_buf = self.blend_buf.last().unwrap();
        
        T::pack(region, blend_buf);
    }

    pub(crate) fn run_cmd(&mut self, cmd: &Cmd, alphas: &[u8], paints: &[EncodedPaint]) {
        match cmd {
            Cmd::Fill(f) => {
                self.fill(
                    usize::from(f.x),
                    usize::from(f.width),
                    &f.paint,
                    f.blend_mode
                        .unwrap_or(BlendMode::new(Mix::Normal, Compose::SrcOver)),
                    paints,
                );
            }
            Cmd::AlphaFill(s) => {
                let a_slice = &alphas[s.alpha_idx..];
                self.strip(
                    usize::from(s.x),
                    usize::from(s.width),
                    a_slice,
                    &s.paint,
                    s.blend_mode
                        .unwrap_or(BlendMode::new(Mix::Normal, Compose::SrcOver)),
                    paints,
                );
            }
            Cmd::PushBuf => {
                self.blend_buf.push([T::Numeric::ZERO; crate::fine::SCRATCH_BUF_SIZE]);
            }
            Cmd::PopBuf => {
                self.blend_buf.pop();
            }
            Cmd::ClipFill(cf) => {
            }
            Cmd::ClipStrip(cs) => {
            }
            _ => unimplemented!()
        }
    }

    #[inline(always)]
    /// Fill at a given x and with a width using the given paint.
    pub fn fill(
        &mut self,
        x: usize,
        width: usize,
        fill: &Paint,
        blend_mode: BlendMode,
        _: &[EncodedPaint],
    ) {
        let blend_buf = &mut self.blend_buf.last_mut().unwrap()[x * TILE_HEIGHT_COMPONENTS..]
            [..TILE_HEIGHT_COMPONENTS * width];
        
        let default_blend = blend_mode == BlendMode::new(Mix::Normal, Compose::SrcOver);

        match fill {
            Paint::Solid(color) => {
                let color = T::extract_color(*color);

                // If color is completely opaque, we can just directly override
                // the blend buffer.
                if color[3] == T::Numeric::ONE && default_blend {
                    T::fill_buf_solid(self.simd, blend_buf, color);

                    return;
                }

                T::fill_solid(self.simd, blend_buf, color, blend_mode);
            }
            Paint::Indexed(paint) => {
                unimplemented!()
            }
        }
    }

    /// Strip at a given x and with a width using the given paint and alpha values.
    #[inline(always)]
    pub fn strip(
        &mut self,
        x: usize,
        width: usize,
        alphas: &[u8],
        fill: &Paint,
        blend_mode: BlendMode,
        encoded_paints: &[EncodedPaint],
    ) {
        debug_assert!(
            alphas.len() >= width,
            "alpha buffer doesn't contain sufficient elements"
        );

        let blend_buf = &mut self.blend_buf.last_mut().unwrap()[x * TILE_HEIGHT_COMPONENTS..]
            [..TILE_HEIGHT_COMPONENTS * width];

        match fill {
            Paint::Solid(color) => {
                T::strip_solid(self.simd, blend_buf, T::extract_color(*color), blend_mode, alphas);
            }
            Paint::Indexed(paint) => {
                let color_buf = &mut self.paint_buf[x * TILE_HEIGHT_COMPONENTS..]
                    [..TILE_HEIGHT_COMPONENTS * width];

                let encoded_paint = &encoded_paints[paint.index()];

                let start_x = self.wide_coords.0 * WideTile::WIDTH + x as u16;
                let start_y = self.wide_coords.1 * Tile::HEIGHT;

                match encoded_paint {
                    EncodedPaint::BlurredRoundedRect(b) => {
                        // let filler = BlurredRoundedRectFiller::new(self.simd, b, start_x, start_y);
                        // T::fill_shader(self.simd, )
                        // fill_complex_paint::<N>(color_buf, blend_buf, true, blend_mode, filler);
                    }
                    _ => unimplemented!()
                }
            }
        }
    }
}

pub trait PosExt<S: Simd> {
    fn splat_x_col_pos(
        simd: S,
        pos: f32,
        x_advance: f32,
        y_advance: f32,
    ) -> Self;
    fn splat_y_col_pos(
        simd: S,
        pos: f32,
        x_advance: f32,
        y_advance: f32,
    ) -> Self;
}

impl<S: Simd> PosExt<S> for f32x4<S> {
    #[inline(always)]
    fn splat_x_col_pos(simd: S, pos: f32, _: f32, _: f32) -> Self {
        f32x4::splat(simd, pos)
    }

    #[inline(always)]
    fn splat_y_col_pos(simd: S, pos: f32, _: f32, y_advance: f32) -> Self {
        let column_mask: f32x4<_> = [0.0, 1.0, 2.0, 3.0].simd_into(simd);
        
        f32x4::splat(simd, pos).madd(column_mask, f32x4::splat(simd, y_advance))
    }
}

impl<S: Simd> PosExt<S> for f32x8<S> {
    #[inline(always)]
    fn splat_x_col_pos(simd: S, pos: f32, x_advance: f32, y_advance: f32) -> Self {
        simd.combine_f32x4(f32x4::splat_x_col_pos(simd, pos, x_advance, y_advance), f32x4::splat_x_col_pos(simd, pos + x_advance, x_advance, y_advance))
    }

    #[inline(always)]
    fn splat_y_col_pos(simd: S, pos: f32, x_advance: f32, y_advance: f32) -> Self {
        simd.combine_f32x4(f32x4::splat_y_col_pos(simd, pos, x_advance, y_advance), f32x4::splat_y_col_pos(simd, pos + x_advance, x_advance, y_advance))
    }
}