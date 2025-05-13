// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Fine rasterization runs the commands in each wide tile to determine the final RGBA value
//! of each pixel and pack it into the pixmap.
use crate::util::scalar::div_255;
use alloc::vec;
use alloc::vec::Vec;
use core::fmt::Debug;
use core::iter;
use std::marker::PhantomData;
use std::ops::{Add, Div, Mul, Sub};
use vello_common::encode::{EncodedKind, EncodedPaint};
use vello_common::paint::{Paint, PremulColor};
use vello_common::peniko::{BlendMode, Compose, Mix};
use vello_common::{
    coarse::{Cmd, WideTile},
    tile::Tile,
};
use vello_simd::{Scalar, Type, Widened, neon, scalar};

pub(crate) const COLOR_COMPONENTS: usize = 4;
pub(crate) const TILE_HEIGHT_COMPONENTS: usize = Tile::HEIGHT as usize * COLOR_COMPONENTS;
#[doc(hidden)]
pub const SCRATCH_BUF_SIZE: usize =
    WideTile::WIDTH as usize * Tile::HEIGHT as usize * COLOR_COMPONENTS;

pub type ScratchBuf<F> = [F; SCRATCH_BUF_SIZE];

#[derive(Debug)]
#[doc(hidden)]
/// This is an internal struct, do not access directly.
pub struct Fine<N: Type + SimdExt> {
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) wide_coords: (u16, u16),
    pub(crate) blend_buf: Vec<ScratchBuf<N::Scalar>>,
    pub(crate) color_buf: ScratchBuf<N::Scalar>,
    phantom_data: PhantomData<N>,
}

impl<N: Type + SimdExt> Fine<N> {
    pub fn new(width: u16, height: u16) -> Self {
        let blend_buf = [N::Scalar::ZERO; SCRATCH_BUF_SIZE];
        let color_buf = [N::Scalar::ZERO; SCRATCH_BUF_SIZE];

        Self {
            width,
            height,
            wide_coords: (0, 0),
            blend_buf: vec![blend_buf],
            color_buf,
            phantom_data: PhantomData::default(),
        }
    }

    pub fn set_coords(&mut self, x: u16, y: u16) {
        self.wide_coords = (x, y);
    }

    pub fn clear(&mut self, premul_color: &PremulColor) {
        let c = premul_color.as_premul_f32().components;
        let blend_buf = self.blend_buf.last_mut().unwrap();

        // if c[0] == c[1]
        //     && c[1] == c[2]
        //     && c[2] == c[3]
        // {
        //     // All components are the same, so we can use memset instead.
        //     blend_buf.fill(c[0]);
        // } else {
        //     let loaded = N::splat_color(premul_color);
        //     for z in blend_buf.array_chunks_mut::<C>() {
        //         loaded.store(z)
        //     }
        // }

        let loaded = N::splat_color(premul_color);
        for z in blend_buf.chunks_exact_mut(N::LENGTH) {
            loaded.store(z)
        }
    }

    #[doc(hidden)]
    pub fn pack(&mut self, out_buf: &mut [u8]) {
        let blend_buf = self.blend_buf.last_mut().unwrap();

        pack::<N::Scalar>(
            out_buf,
            blend_buf,
            self.width.into(),
            self.height.into(),
            self.wide_coords.0.into(),
            self.wide_coords.1.into(),
        );
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
                self.blend_buf.push([N::Scalar::ZERO; SCRATCH_BUF_SIZE]);
            }
            Cmd::PopBuf => {
                self.blend_buf.pop();
            }
            Cmd::ClipFill(cf) => {
                self.clip_fill(cf.x as usize, cf.width as usize);
            }
            Cmd::ClipStrip(cs) => {
                let aslice = &alphas[cs.alpha_idx..];
                self.clip_strip(cs.x as usize, cs.width as usize, aslice);
            }
            Cmd::Blend(_) => {
                unimplemented!()
            }
            Cmd::Opacity(_) => {
                unimplemented!()
            }
            Cmd::Mask(_) => {
                unimplemented!()
            }
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
                let has_alpha = color.as_premul_f32().components[3] == 1.0;

                // If color is completely opaque we can just memcopy the colors.
                if has_alpha && default_blend {
                    let color = N::splat_color(color);

                    for t in blend_buf.chunks_exact_mut(N::LENGTH) {
                        color.store(t);
                    }

                    return;
                }

                fill::alpha_composite::<N>(blend_buf, color);
            }
            Paint::Indexed(_) => unimplemented!(),
        }
    }

    /// Strip at a given x and with a width using the given paint and alpha values.
    pub fn strip(
        &mut self,
        x: usize,
        width: usize,
        alphas: &[u8],
        fill: &Paint,
        _: BlendMode,
        _: &[EncodedPaint],
    ) {
        debug_assert!(
            alphas.len() >= width,
            "alpha buffer doesn't contain sufficient elements"
        );

        let blend_buf = &mut self.blend_buf.last_mut().unwrap()[x * TILE_HEIGHT_COMPONENTS..]
            [..TILE_HEIGHT_COMPONENTS * width];

        match fill {
            Paint::Solid(color) => {
                strip::alpha_composite::<N>(blend_buf, color, alphas);
            }
            Paint::Indexed(_) => unimplemented!(),
        }
    }

    fn clip_fill(&mut self, x: usize, width: usize) {
        unimplemented!()
    }

    fn clip_strip(&mut self, x: usize, width: usize, alphas: &[u8]) {
        unimplemented!()
    }
}

fn pack<F: Scalar>(
    out_buf: &mut [u8],
    scratch: &ScratchBuf<F>,
    width: usize,
    height: usize,
    x: usize,
    y: usize,
) {
    let base_ix = (y * usize::from(Tile::HEIGHT) * width + x * usize::from(WideTile::WIDTH))
        * COLOR_COMPONENTS;

    // Make sure we don't process rows outside the range of the pixmap.
    let max_height = (height - y * usize::from(Tile::HEIGHT)).min(usize::from(Tile::HEIGHT));

    for j in 0..max_height {
        let line_ix = base_ix + j * width * COLOR_COMPONENTS;

        // Make sure we don't process columns outside the range of the pixmap.
        let max_width =
            (width - x * usize::from(WideTile::WIDTH)).min(usize::from(WideTile::WIDTH));
        let target_len = max_width * COLOR_COMPONENTS;
        // This helps the compiler to understand that any access to `dest` cannot
        // be out of bounds, and thus saves corresponding checks in the for loop.
        let dest = &mut out_buf[line_ix..][..target_len];

        for i in 0..max_width {
            let src = &scratch[(i * usize::from(Tile::HEIGHT) + j) * COLOR_COMPONENTS..]
                [..COLOR_COMPONENTS];
            dest[i * COLOR_COMPONENTS..][..COLOR_COMPONENTS].copy_from_slice(&F::to_rgba8(src));
        }
    }
}

pub(crate) mod fill {
    use crate::fine2::SimdExt;
    use vello_common::paint::PremulColor;
    use vello_simd::{Scalar, Type};

    #[inline(never)]
    pub(crate) fn alpha_composite<N: Type + SimdExt>(
        target: &mut [N::Scalar],
        src_c: &PremulColor,
    ) {
        let one_minus_alpha = N::splat_alpha(src_c).one_minus();
        let src_c = N::splat_color(src_c);

        for part in target.chunks_exact_mut(N::LENGTH) {
            let mut bg_c = N::load(part);
            bg_c = bg_c.normalized_mul_add(one_minus_alpha, src_c);
            bg_c.store(part);
        }
    }
}

pub(crate) mod strip {
    use crate::fine2::{COLOR_COMPONENTS, SimdExt, TILE_HEIGHT_COMPONENTS};
    use vello_common::paint::PremulColor;
    use vello_simd::{Scalar, Type, Widened};

    #[inline(never)]
    pub(crate) fn alpha_composite<N: Type + SimdExt>(
        target: &mut [N::Scalar],
        src_c: &PremulColor,
        alphas: &[u8],
    ) {
        let src_alpha = N::splat_alpha(src_c);
        let src_c = N::splat_color(src_c);

        for (bg_part, masks) in target
            .chunks_exact_mut(N::LENGTH)
            .zip(alphas.chunks_exact(N::LENGTH / 4))
        {
            let bg_c = N::load(bg_part);
            let mask_a = N::load_alphas(masks);
            let inv_src_a_mask_a = mask_a.normalized_mul_sub(src_alpha, N::one());

            let res = bg_c.normalized_mul_mul_add(inv_src_a_mask_a, src_c, mask_a);
            res.store(bg_part);
        }
    }
}

pub trait SimdExt {
    fn splat_color(color: &PremulColor) -> Self;
    fn splat_alpha(color: &PremulColor) -> Self;
}

impl SimdExt for scalar::Integer {
    fn splat_color(color: &PremulColor) -> Self {
        Self::load_4(&color.as_premul_rgba8().to_u8_array())
    }

    fn splat_alpha(color: &PremulColor) -> Self {
        Self::splat(color.as_premul_rgba8().a)
    }
}

impl SimdExt for neon::Integer {
    fn splat_color(color: &PremulColor) -> Self {
        Self::load_4(&color.as_premul_rgba8().to_u8_array())
    }

    fn splat_alpha(color: &PremulColor) -> Self {
        Self::splat(color.as_premul_rgba8().a)
    }
}

impl SimdExt for scalar::Float {
    fn splat_color(color: &PremulColor) -> Self {
        Self::load_4(&color.as_premul_f32().components)
    }

    fn splat_alpha(color: &PremulColor) -> Self {
        Self::splat(color.as_premul_f32().components[3])
    }
}

impl SimdExt for neon::Float {
    fn splat_color(color: &PremulColor) -> Self {
        Self::load_4(&color.as_premul_f32().components)
    }

    fn splat_alpha(color: &PremulColor) -> Self {
        Self::splat(color.as_premul_f32().components[3])
    }
}
