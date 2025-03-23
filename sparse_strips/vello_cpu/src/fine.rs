// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Fine rasterization runs the commands in each wide tile to determine the final RGBA value
//! of each pixel and pack it into the pixmap.

use crate::paint::{EncodedLinearGradient, EncodedPaint};
use crate::util::ColorExt;
use std::iter;
use vello_common::{
    coarse::{Cmd, WideTile},
    paint::Paint,
    tile::Tile,
};

pub(crate) const COLOR_COMPONENTS: usize = 4;
pub(crate) const TILE_HEIGHT_COMPONENTS: usize = Tile::HEIGHT as usize * COLOR_COMPONENTS;
pub(crate) const SCRATCH_BUF_SIZE: usize =
    WideTile::WIDTH as usize * Tile::HEIGHT as usize * COLOR_COMPONENTS;

pub(crate) type ScratchBuf = [u8; SCRATCH_BUF_SIZE];

#[derive(Debug)]
#[doc(hidden)]
/// This is an internal struct, do not access directly.
pub struct Fine<'a> {
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) out_buf: &'a mut [u8],
    pub(crate) blend_buf: ScratchBuf,
    pub(crate) color_buf: ScratchBuf,
}

impl<'a> Fine<'a> {
    /// Create a new fine rasterizer.
    pub fn new(width: u16, height: u16, out_buf: &'a mut [u8]) -> Self {
        let scratch = [0; SCRATCH_BUF_SIZE];
        let color_scratch = [0; SCRATCH_BUF_SIZE];

        Self {
            width,
            height,
            out_buf,
            blend_buf: scratch,
            color_buf: color_scratch,
        }
    }

    pub(crate) fn clear(&mut self, premul_color: [u8; 4]) {
        if premul_color[0] == premul_color[1]
            && premul_color[1] == premul_color[2]
            && premul_color[2] == premul_color[3]
        {
            // All components are the same, so we can use memset instead.
            self.blend_buf.fill(premul_color[0]);
        } else {
            for z in self.blend_buf.chunks_exact_mut(COLOR_COMPONENTS) {
                z.copy_from_slice(&premul_color);
            }
        }
    }

    pub(crate) fn pack(&mut self, x: u16, y: u16) {
        pack(
            self.out_buf,
            &self.blend_buf,
            self.width.into(),
            self.height.into(),
            x.into(),
            y.into(),
        );
    }

    pub(crate) fn run_cmd(&mut self, tile_x: u16, tile_y: u16, cmd: &Cmd, alphas: &[u8]) {
        match cmd {
            Cmd::Fill(f) => {
                self.fill(
                    f.x as usize,
                    tile_x,
                    tile_y,
                    f.width as usize,
                    &f.paint.clone().into(),
                );
            }
            Cmd::AlphaFill(s) => {
                let a_slice = &alphas[s.alpha_ix..];
                self.strip(
                    s.x as usize,
                    tile_x,
                    tile_y,
                    s.width as usize,
                    a_slice,
                    &s.paint.clone().into(),
                );
            }
        }
    }

    /// Fill at a given x and with a width using the given paint.
    pub fn fill(&mut self, x: usize, tile_x: u16, tile_y: u16, width: usize, paint: &EncodedPaint) {
        let blend_buf =
            &mut self.blend_buf[x * TILE_HEIGHT_COMPONENTS..][..TILE_HEIGHT_COMPONENTS * width];
        let color_buf =
            &mut self.color_buf[x * TILE_HEIGHT_COMPONENTS..][..TILE_HEIGHT_COMPONENTS * width];

        match paint {
            EncodedPaint::Solid(color) => {
                // If color is completely opaque we can just memcopy the colors.
                if color[3] == 255 {
                    for t in blend_buf.chunks_exact_mut(COLOR_COMPONENTS) {
                        t.copy_from_slice(color);
                    }

                    return;
                }

                fill::src_over(blend_buf, iter::repeat(*color));
            }
            EncodedPaint::LinearGradient(g) => {
                let start_x = tile_x * WideTile::WIDTH + x as u16;
                let start_y = tile_y * Tile::HEIGHT;
                let mut iter = GradientFiller::new(g, start_x, start_y);

                if g.has_opacities {
                    iter.run(color_buf);
                    fill::src_over(
                        blend_buf,
                        color_buf.chunks_exact(4).map(|e| [e[0], e[1], e[2], e[3]]),
                    );
                } else {
                    // Similarly to solid colors we can just override the previous values
                    // if all colors in the gradient are fully opaque.
                    iter.run(blend_buf);
                }
            }
        }
    }

    /// Strip at a given x and with a width using the given paint and alpha values.
    pub fn strip(
        &mut self,
        x: usize,
        tile_x: u16,
        tile_y: u16,
        width: usize,
        alphas: &[u8],
        paint: &EncodedPaint,
    ) {
        debug_assert!(
            alphas.len() >= width,
            "alpha buffer doesn't contain sufficient elements"
        );

        let blend_buf =
            &mut self.blend_buf[x * TILE_HEIGHT_COMPONENTS..][..TILE_HEIGHT_COMPONENTS * width];
        let color_buf =
            &mut self.color_buf[x * TILE_HEIGHT_COMPONENTS..][..TILE_HEIGHT_COMPONENTS * width];

        match paint {
            EncodedPaint::Solid(color) => {
                strip::src_over(blend_buf, iter::repeat(*color), alphas);
            }
            EncodedPaint::LinearGradient(g) => {
                let start_x = tile_x * WideTile::WIDTH + x as u16;
                let start_y = tile_y * Tile::HEIGHT;
                let mut iter = GradientFiller::new(g, start_x, start_y);
                iter.run(color_buf);
                strip::src_over(
                    blend_buf,
                    color_buf.chunks_exact(4).map(|e| [e[0], e[1], e[2], e[3]]),
                    alphas,
                );
            }
        }
    }
}

fn pack(out_buf: &mut [u8], scratch: &ScratchBuf, width: usize, height: usize, x: usize, y: usize) {
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
            dest[i * COLOR_COMPONENTS..][..COLOR_COMPONENTS]
                .copy_from_slice(&src[..COLOR_COMPONENTS]);
        }
    }
}

pub(crate) mod fill {
    // See https://www.w3.org/TR/compositing-1/#porterduffcompositingoperators for the
    // formulas.

    use crate::fine::{COLOR_COMPONENTS, TILE_HEIGHT_COMPONENTS};
    use crate::util::scalar::div_255;

    pub(crate) fn src_over<T: Iterator<Item = [u8; COLOR_COMPONENTS]>>(
        target: &mut [u8],
        mut color_iter: T,
    ) {
        for strip in target.chunks_exact_mut(TILE_HEIGHT_COMPONENTS) {
            for bg_c in strip.chunks_exact_mut(COLOR_COMPONENTS) {
                let src_c = color_iter.next().unwrap();
                for i in 0..COLOR_COMPONENTS {
                    bg_c[i] = src_c[i] + div_255(bg_c[i] as u16 * (255 - src_c[3] as u16)) as u8;
                }
            }
        }
    }
}

pub(crate) mod strip {
    use crate::fine::{COLOR_COMPONENTS, TILE_HEIGHT_COMPONENTS};
    use crate::util::scalar::div_255;
    use vello_common::tile::Tile;

    pub(crate) fn src_over<T: Iterator<Item = [u8; COLOR_COMPONENTS]>>(
        target: &mut [u8],
        mut color_iter: T,
        alphas: &[u8],
    ) {
        for (bg_c, masks) in target
            .chunks_exact_mut(TILE_HEIGHT_COMPONENTS)
            .zip(alphas.chunks_exact(usize::from(Tile::HEIGHT)))
        {
            for j in 0..usize::from(Tile::HEIGHT) {
                let src_c = color_iter.next().unwrap();
                let mask_a = u16::from(masks[j]);
                let inv_src_a_mask_a = 255 - div_255(mask_a * src_c[3] as u16);

                for i in 0..COLOR_COMPONENTS {
                    let im1 = bg_c[j * COLOR_COMPONENTS + i] as u16 * inv_src_a_mask_a;
                    let im2 = src_c[i] as u16 * mask_a;
                    let im3 = div_255(im1 + im2);
                    bg_c[j * COLOR_COMPONENTS + i] = im3 as u8;
                }
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct GradientFiller<'a> {
    /// The position of the next x that should be processed.
    cur_pos: f32,
    x_advance: f32,
    y_advance: f32,
    /// The index of the current right stop we are processing.
    stop_idx: usize,
    /// The x-position of the left stop.
    x0: f32,
    /// The x-position of the right stop.
    x1: f32,
    /// The color of the left stop.
    c0: [u8; 4],
    /// The color of the right stop.
    c1: [u8; 4],
    im1: [f32; 4],
    im2: f32,
    im3: [f32; 4],
    /// The output buffer for emitting colors from the iterator.
    color_buf: [u8; COLOR_COMPONENTS],
    /// The underlying gradient.
    gradient: &'a EncodedLinearGradient,
}

impl<'a> GradientFiller<'a> {
    pub(crate) fn new(
        gradient: &'a EncodedLinearGradient,
        mut start_x: u16,
        mut start_y: u16,
    ) -> Self {
        // The actual starting point of the strip.
        let x0 = start_x as f32 + gradient.offsets.0;
        let y0 = start_y as f32 + gradient.offsets.1;

        let cur_pos = (x0 * gradient.fact1 - y0 * gradient.fact2) / gradient.denom;

        let mut filler = Self {
            cur_pos,
            x_advance: gradient.advances.0,
            y_advance: gradient.advances.1,
            stop_idx: gradient.stops.len(),
            x0: 0.0,
            x1: 0.0,
            c0: [0; 4],
            c1: [0; 4],
            im1: [0.0; 4],
            im2: 0.0,
            im3: [0.0; 4],
            color_buf: [0; COLOR_COMPONENTS],
            gradient,
        };

        filler.advance();

        filler
    }
}

impl GradientFiller<'_> {
    fn advance(&mut self) {
        self.stop_idx += 1;

        if self.stop_idx >= self.gradient.stops.len() {
            self.stop_idx = 1;
        }

        let left_stop = &self.gradient.stops[self.stop_idx - 1];
        let right_stop = &self.gradient.stops[self.stop_idx];

        self.x0 = self.gradient.end * left_stop.offset;
        self.x1 = self.gradient.end * right_stop.offset;
        self.c0 = left_stop.color;
        self.c1 = right_stop.color;

        self.im2 = self.x1 - self.x0;

        for i in 0..COLOR_COMPONENTS {
            self.im1[i] = self.c1[i] as f32 - self.c0[i] as f32;
            self.im3[i] = self.im1[i] / self.im2;
        }
    }

    fn run(mut self, target: &mut [u8]) {
        target
            .chunks_exact_mut(TILE_HEIGHT_COMPONENTS)
            .for_each(|col| {
                let mut cur_pos = self.cur_pos;

                for pixel in col.chunks_exact_mut(COLOR_COMPONENTS) {
                    let target_pos = if self.gradient.pad {
                        cur_pos.clamp(0.0, self.gradient.end)
                    } else {
                        cur_pos.rem_euclid(self.gradient.end)
                    };

                    // It's possible that we have to skip multiple stops.
                    while target_pos > self.x1 || target_pos < self.x0 {
                        self.advance();
                    }

                    for col_idx in 0..COLOR_COMPONENTS {
                        let im3 = target_pos - self.x0;
                        let combined = (self.im3[col_idx] * im3 + 0.5) as i16;

                        pixel[col_idx] = (self.c0[col_idx] as i16 + combined) as u8;
                    }

                    cur_pos += self.y_advance;
                }

                self.cur_pos += self.x_advance;
            })
    }
}
