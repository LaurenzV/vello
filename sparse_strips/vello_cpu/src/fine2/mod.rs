mod highp;
mod lowp;

use core::fmt::Debug;
use std::iter;
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

pub type ScratchBuf<F> = [F; SCRATCH_BUF_SIZE];

pub trait Numeric: Copy + Default + Clone + Debug + PartialEq {
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

pub trait FineKernel {
    type Numeric: Numeric;
    
    fn extract_color(color: PremulColor) -> [Self::Numeric; 4];
    fn pack(region: &mut Region<'_>, blend_buf: &[Self::Numeric]);
    fn fill_solid(
        level: Level,
        target: &mut [Self::Numeric],
        color: [Self::Numeric; 4],
        blend_mode: BlendMode,
    );
}


#[derive(Debug)]
pub struct Fine<T: FineKernel> {
    level: Level,
    pub(crate) wide_coords: (u16, u16),
    pub(crate) blend_buf: Vec<ScratchBuf<T::Numeric>>,
    pub(crate) paint_buf: ScratchBuf<T::Numeric>,
}

impl<T: FineKernel> Fine<T> {
    pub fn new() -> Self {
        Self {
            level: Level::new(),
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
                // self.strip(
                //     usize::from(s.x),
                //     usize::from(s.width),
                //     a_slice,
                //     &s.paint,
                //     s.blend_mode
                //         .unwrap_or(BlendMode::new(Mix::Normal, Compose::SrcOver)),
                //     paints,
                // );
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

                // If color is completely opaque we can just memcopy the colors.
                if color[3] == T::Numeric::ONE && default_blend {
                    for t in blend_buf.chunks_exact_mut(COLOR_COMPONENTS) {
                        // TODO: Faster with 512x SIMD?
                        t.copy_from_slice(&color);
                    }

                    return;
                }

                T::fill_solid(self.level, blend_buf, color, blend_mode);
            }
            Paint::Indexed(paint) => {
                unimplemented!()
            }
        }
    }
}