// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Fine rasterization runs the commands in each wide tile to determine the final RGBA value
//! of each pixel and pack it into the pixmap.

mod rounded_blurred_rect;

use crate::fine2::rounded_blurred_rect::BlurredRoundedRectFiller;
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
use vello_simd::{NumberKind, Type, Widened, fallback, neon};

pub(crate) const COLOR_COMPONENTS: usize = 4;
pub(crate) const TILE_HEIGHT_COMPONENTS: usize = Tile::HEIGHT as usize * COLOR_COMPONENTS;
#[doc(hidden)]
pub const SCRATCH_BUF_SIZE: usize =
    WideTile::WIDTH as usize * Tile::HEIGHT as usize * COLOR_COMPONENTS;

pub type ScratchBuf<F> = [F; SCRATCH_BUF_SIZE];

#[derive(Debug)]
#[doc(hidden)]
/// This is an internal struct, do not access directly.
pub struct Fine<N: Type> {
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) wide_coords: (u16, u16),
    pub(crate) blend_buf: Vec<ScratchBuf<N::Scalar>>,
    pub(crate) color_buf: ScratchBuf<N::Scalar>,
    phantom_data: PhantomData<N>,
}

impl<N: Type> Fine<N> {
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
        let blend_buf = self.blend_buf.last_mut().unwrap();

        let loaded = N::splat_color(*premul_color);
        for z in blend_buf.chunks_exact_mut(N::LENGTH) {
            loaded.store(z)
        }
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
        encoded_paints: &[EncodedPaint],
    ) {
        let blend_buf = &mut self.blend_buf.last_mut().unwrap()[x * TILE_HEIGHT_COMPONENTS..]
            [..TILE_HEIGHT_COMPONENTS * width];

        let default_blend = blend_mode == BlendMode::new(Mix::Normal, Compose::SrcOver);

        fn fill_complex_paint<T: Type>(
            color_buf: &mut [T::Scalar],
            blend_buf: &mut [T::Scalar],
            has_opacities: bool,
            filler: impl Painter<T>,
        ) {
            if has_opacities {
                filler.paint(color_buf);
                fill::alpha_composite(
                    blend_buf,
                    color_buf.chunks_exact(T::LENGTH).map(|e| T::load(e)),
                );
            } else {
                // Similarly to solid colors we can just override the previous values
                // if all colors in the gradient are fully opaque.
                filler.paint(blend_buf);
            }
        }

        match fill {
            Paint::Solid(color) => {
                let has_alpha = color.as_premul_f32().components[3] == 1.0;

                // If color is completely opaque we can just memcopy the colors.
                if has_alpha && default_blend {
                    let color = N::splat_color(*color);

                    for t in blend_buf.chunks_exact_mut(N::LENGTH) {
                        color.store(t);
                    }

                    return;
                }

                fill::alpha_composite_solid::<N>(blend_buf, color);
            }
            Paint::Indexed(paint) => {
                let color_buf = &mut self.color_buf[x * TILE_HEIGHT_COMPONENTS..]
                    [..TILE_HEIGHT_COMPONENTS * width];

                let encoded_paint = &encoded_paints[paint.index()];

                let start_x = self.wide_coords.0 * WideTile::WIDTH + x as u16;
                let start_y = self.wide_coords.1 * Tile::HEIGHT;

                match encoded_paint {
                    EncodedPaint::BlurredRoundedRect(b) => {
                        let filler = BlurredRoundedRectFiller::new(b, start_x, start_y);
                        fill_complex_paint::<N>(color_buf, blend_buf, true, filler);
                    }
                    _ => unimplemented!(),
                }
            }
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
        encoded_paints: &[EncodedPaint],
    ) {
        debug_assert!(
            alphas.len() >= width,
            "alpha buffer doesn't contain sufficient elements"
        );

        let blend_buf = &mut self.blend_buf.last_mut().unwrap()[x * TILE_HEIGHT_COMPONENTS..]
            [..TILE_HEIGHT_COMPONENTS * width];

        fn strip_complex_paint<F: Type>(
            color_buf: &mut [F::Scalar],
            blend_buf: &mut [F::Scalar],
            filler: impl Painter<F>,
            alphas: &[u8],
        ) {
            filler.paint(color_buf);

            strip::alpha_composite(
                blend_buf,
                color_buf.chunks_exact(F::LENGTH).map(|e| F::load(e)),
                alphas,
            );
        }

        match fill {
            Paint::Solid(color) => {
                strip::alpha_composite_solid::<N>(blend_buf, color, alphas);
            }
            Paint::Indexed(paint) => {
                let encoded_paint = &encoded_paints[paint.index()];

                let color_buf = &mut self.color_buf[x * TILE_HEIGHT_COMPONENTS..]
                    [..TILE_HEIGHT_COMPONENTS * width];

                let start_x = self.wide_coords.0 * WideTile::WIDTH + x as u16;
                let start_y = self.wide_coords.1 * Tile::HEIGHT;

                match encoded_paint {
                    EncodedPaint::BlurredRoundedRect(b) => {
                        let filler = BlurredRoundedRectFiller::new(b, start_x, start_y);
                        strip_complex_paint::<N>(color_buf, blend_buf, filler, alphas);
                    }
                    _ => unimplemented!(),
                }
            }
        }
    }

    #[doc(hidden)]
    pub fn pack(&mut self, out_buf: &mut [u8]) {
        let blend_buf = self.blend_buf.last_mut().unwrap();
        let (x, y) = (self.wide_coords.0 as usize, self.wide_coords.1 as usize);
        let (width, height) = (self.width as usize, self.height as usize);

        N::pack(out_buf, blend_buf, x, y, width, height);
    }

    fn clip_fill(&mut self, x: usize, width: usize) {
        let (source_buffer, rest) = self.blend_buf.split_last_mut().unwrap();
        let target_buffer = rest.last_mut().unwrap();

        let source_buffer =
            &mut source_buffer[x * TILE_HEIGHT_COMPONENTS..][..TILE_HEIGHT_COMPONENTS * width];
        let target_buffer =
            &mut target_buffer[x * TILE_HEIGHT_COMPONENTS..][..TILE_HEIGHT_COMPONENTS * width];

        fill::alpha_composite(
            target_buffer,
            source_buffer.chunks_exact(N::LENGTH).map(|e| N::load(e)),
        );
    }

    fn clip_strip(&mut self, x: usize, width: usize, alphas: &[u8]) {
        let (source_buffer, rest) = self.blend_buf.split_last_mut().unwrap();
        let target_buffer = rest.last_mut().unwrap();

        let source_buffer =
            &mut source_buffer[x * TILE_HEIGHT_COMPONENTS..][..TILE_HEIGHT_COMPONENTS * width];
        let target_buffer =
            &mut target_buffer[x * TILE_HEIGHT_COMPONENTS..][..TILE_HEIGHT_COMPONENTS * width];

        strip::alpha_composite(
            target_buffer,
            source_buffer.chunks_exact(N::LENGTH).map(|e| N::load(e)),
            alphas,
        );
    }
}

pub(crate) mod fill {
    use vello_common::paint::PremulColor;
    use vello_simd::{NumberKind, Type};

    pub(crate) fn alpha_composite_solid<N: Type>(target: &mut [N::Scalar], src_c: &PremulColor) {
        let one_minus_alpha = N::splat_alpha(*src_c).one_minus();
        let src_c = N::splat_color(*src_c);

        for part in target.chunks_exact_mut(N::LENGTH) {
            alpha_composite_inner(part, src_c, one_minus_alpha)
        }
    }

    pub(crate) fn alpha_composite<N: Type, T: Iterator<Item = N>>(
        target: &mut [N::Scalar],
        src_c: T,
    ) {
        for (part, src_c) in target.chunks_exact_mut(N::LENGTH).zip(src_c) {
            let one_minus_alpha = src_c.splat_4th_element().one_minus();
            alpha_composite_inner(part, src_c, one_minus_alpha)
        }
    }

    #[inline(always)]
    fn alpha_composite_inner<N: Type>(target: &mut [N::Scalar], src_c: N, one_minus_alpha: N) {
        let mut bg_c = N::load(target);
        bg_c = bg_c.normalized_mul_add(one_minus_alpha, src_c);
        bg_c.store(target);
    }
}

pub(crate) mod strip {
    use vello_common::paint::PremulColor;
    use vello_simd::Type;

    pub(crate) fn alpha_composite_solid<N: Type>(
        target: &mut [N::Scalar],
        src_c: &PremulColor,
        alphas: &[u8],
    ) {
        let src_a = N::splat_alpha(*src_c);
        let src_c = N::splat_color(*src_c);
        let one = N::one();

        for (bg_part, masks) in target
            .chunks_exact_mut(N::LENGTH)
            .zip(alphas.chunks_exact(N::LENGTH / 4))
        {
            // Not passing the `one` explicitly here messes with auto-vectorization.
            alpha_composite_inner(bg_part, masks, src_c, src_a, one);
        }
    }

    pub(crate) fn alpha_composite<N: Type, T: Iterator<Item = N>>(
        target: &mut [N::Scalar],
        src_c: T,
        alphas: &[u8],
    ) {
        let one = N::one();

        for ((bg_part, masks), src_c) in target
            .chunks_exact_mut(N::LENGTH)
            .zip(alphas.chunks_exact(N::LENGTH / 4))
            .zip(src_c)
        {
            let src_a = src_c.splat_4th_element();
            alpha_composite_inner(bg_part, masks, src_c, src_a, one);
        }
    }

    #[inline(always)]
    fn alpha_composite_inner<N: Type>(
        target: &mut [N::Scalar],
        masks: &[u8],
        src_c: N,
        src_a: N,
        one: N,
    ) {
        let bg_c = N::load(target);
        let mask_a = N::load_alphas(masks);
        let inv_src_a_mask_a = mask_a.normalized_mul_sub(src_a, one);

        let res = bg_c.normalized_mul_mul_add(inv_src_a_mask_a, src_c, mask_a);
        res.store(target);
    }
}

trait Painter<F: Type> {
    fn paint(self, target: &mut [F::Scalar]);
}
