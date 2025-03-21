// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Fine rasterization runs the commands in each wide tile to determine the final RGBA value
//! of each pixel and pack it into the pixmap.

use crate::util::ColorExt;
use std::iter;
use vello_common::paint::InnerLinearGradient;
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
    pub(crate) scratch: ScratchBuf,
}

impl<'a> Fine<'a> {
    /// Create a new fine rasterizer.
    pub fn new(width: u16, height: u16, out_buf: &'a mut [u8]) -> Self {
        let scratch = [0; SCRATCH_BUF_SIZE];

        Self {
            width,
            height,
            out_buf,
            scratch,
        }
    }

    pub(crate) fn clear(&mut self, premul_color: [u8; 4]) {
        if premul_color[0] == premul_color[1]
            && premul_color[1] == premul_color[2]
            && premul_color[2] == premul_color[3]
        {
            // All components are the same, so we can use memset instead.
            self.scratch.fill(premul_color[0]);
        } else {
            for z in self.scratch.chunks_exact_mut(COLOR_COMPONENTS) {
                z.copy_from_slice(&premul_color);
            }
        }
    }

    pub(crate) fn pack(&mut self, x: u16, y: u16) {
        pack(
            self.out_buf,
            &self.scratch,
            self.width.into(),
            self.height.into(),
            x.into(),
            y.into(),
        );
    }

    pub(crate) fn run_cmd(&mut self, tile_x: u16, cmd: &Cmd, alphas: &[u8]) {
        match cmd {
            Cmd::Fill(f) => {
                self.fill(f.x as usize, tile_x, f.width as usize, &f.paint);
            }
            Cmd::AlphaFill(s) => {
                let a_slice = &alphas[s.alpha_ix..];
                self.strip(s.x as usize, tile_x, s.width as usize, a_slice, &s.paint);
            }
        }
    }

    /// Fill at a given x and with a width using the given paint.
    pub fn fill(&mut self, x: usize, tile_x: u16, width: usize, paint: &Paint) {
        let target =
            &mut self.scratch[x * TILE_HEIGHT_COMPONENTS..][..TILE_HEIGHT_COMPONENTS * width];

        match paint {
            Paint::Solid(c) => {
                let color = c.premultiply().to_rgba8_fast();

                // If color is completely opaque we can just memcopy the colors.
                if color[3] == 255 {
                    for t in target.chunks_exact_mut(COLOR_COMPONENTS) {
                        t.copy_from_slice(&color);
                    }

                    return;
                }

                fill::src_over(target, iter::repeat(color));
            }
            Paint::Gradient(g) => {
                let start_x = tile_x * WideTile::WIDTH + x as u16;
                fill::src_over(target, LinearGradientIter::new(g, start_x));
            }
            _ => unimplemented!(),
        }
    }

    /// Strip at a given x and with a width using the given paint and alpha values.
    pub fn strip(&mut self, x: usize, tile_x: u16, width: usize, alphas: &[u8], paint: &Paint) {
        debug_assert!(
            alphas.len() >= width,
            "alpha buffer doesn't contain sufficient elements"
        );

        let target =
            &mut self.scratch[x * TILE_HEIGHT_COMPONENTS..][..TILE_HEIGHT_COMPONENTS * width];

        match paint {
            Paint::Solid(s) => {
                let color = s.premultiply().to_rgba8_fast();

                strip::src_over(target, iter::repeat(color), alphas);
            }
            Paint::Gradient(g) => {
                let start_x = tile_x * WideTile::WIDTH + x as u16;
                strip::src_over(target, LinearGradientIter::new(g, start_x), alphas);
            }
            _ => unimplemented!(),
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
pub(crate) struct LinearGradientIter<'a> {
    /// The position of the next x that should be processed.
    cur_x: f32,
    /// The position of the current column we are generating pixels for.
    col_pos: u16,
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
    /// The output buffer for emitting colors from the iterator.
    color_buf: [u8; COLOR_COMPONENTS],
    /// The underlying gradient.
    gradient: &'a InnerLinearGradient,
}

impl<'a> LinearGradientIter<'a> {
    pub(crate) fn new(gradient: &'a InnerLinearGradient, start_x: u16) -> Self {
        let mut iter = Self {
            cur_x: start_x as f32 - 1.0 + gradient.offset,
            col_pos: Tile::HEIGHT,
            stop_idx: gradient.stops.len(),
            x0: 0.0,
            x1: 0.0,
            c0: [0; 4],
            c1: [0; 4],
            color_buf: [0; COLOR_COMPONENTS],
            gradient,
        };

        iter.advance();

        iter
    }
}

impl LinearGradientIter<'_> {
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
    }
}

impl Iterator for LinearGradientIter<'_> {
    type Item = [u8; COLOR_COMPONENTS];

    fn next(&mut self) -> Option<Self::Item> {
        // For linear gradients with no skewing transform, the color values
        // in a column are always the same, so we can cache them.
        if self.col_pos < (Tile::HEIGHT - 1) {
            self.col_pos += 1;
            return Some(self.color_buf);
        }

        self.col_pos = 0;
        self.cur_x += 1.0;

        let cur_x = self.cur_x.clamp(0.0, self.gradient.end);

        // It's possible that we have to skip multiple stops.
        while cur_x > self.x1 || cur_x < self.x0 {
            self.advance();
        }

        for col_idx in 0..COLOR_COMPONENTS {
            let idx = col_idx;
            let im1 = self.c1[col_idx] as f32 - self.c0[col_idx] as f32;
            let im2 = self.x1 - self.x0;
            let im3 = cur_x - self.x0;
            let combined = ((im1 / im2) * im3 + 0.5) as i16;

            self.color_buf[idx] = (self.c0[col_idx] as i16 + combined) as u8;
        }

        Some(self.color_buf)
    }
}

#[cfg(test)]
mod tests {
    use crate::fine::LinearGradientIter;
    use vello_common::color::palette::css::{BLACK, BLUE, GREEN, WHITE};
    use vello_common::paint::{LinearGradient, Stop};
    use vello_common::peniko::Extend;
    //
    // #[test]
    // fn gradient_iter_1() {
    //     let gradient = LinearGradient {
    //         x0: 10.0,
    //         x1: 15.0,
    //         stops: vec![
    //             Stop {
    //                 offset: 0.0,
    //                 color: WHITE,
    //             },
    //             Stop {
    //                 offset: 1.0,
    //                 color: BLACK,
    //             },
    //         ],
    //         extend: Extend::Pad,
    //     };
    //
    //     let inner = gradient.into();
    //     let mut iter = LinearGradientIter::new(&inner, 10);
    //
    //     for i in 0..20 {
    //         println!("{:?}", iter.next().unwrap());
    //     }
    // }
}
