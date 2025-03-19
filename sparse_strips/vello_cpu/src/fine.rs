// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Fine rasterization runs the commands in each wide tile to determine the final RGBA value
//! of each pixel and pack it into the pixmap.

use crate::util::ColorExt;
use std::iter;
use vello_common::color::palette::css::RED;
use vello_common::paint::{LinearGradient, Stop};
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

pub struct Fine<'a> {
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) out_buf: &'a mut [u8],
    pub(crate) scratch: ScratchBuf,
}

impl<'a> Fine<'a> {
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

    pub(crate) fn run_cmd(&mut self, tile_x: u16, cmd: &Cmd, alphas: &[u32]) {
        match cmd {
            Cmd::Fill(f) => {
                self.fill(f.x as usize, tile_x, f.width as usize, &f.paint);
            }
            Cmd::AlphaFill(s) => {
                let a_slice = &alphas[s.alpha_ix..];
                self.strip(s.x as usize, s.width as usize, a_slice, &s.paint);
            }
        }
    }

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
                fill::src_over_grad(target, tile_x * WideTile::WIDTH + x as u16, g);
            }
            _ => unimplemented!(),
        }
    }

    pub(crate) fn strip(&mut self, x: usize, width: usize, alphas: &[u32], paint: &Paint) {
        debug_assert!(
            alphas.len() >= width,
            "alpha buffer doesn't contain sufficient elements"
        );

        match paint {
            Paint::Solid(s) => {
                let color = s.premultiply().to_rgba8_fast();

                let target = &mut self.scratch[x * TILE_HEIGHT_COMPONENTS..]
                    [..TILE_HEIGHT_COMPONENTS * width];

                strip::src_over(target, &color, alphas);
            }
            _ => {
                let color = RED.premultiply().to_rgba8_fast();

                let target = &mut self.scratch[x * TILE_HEIGHT_COMPONENTS..]
                    [..TILE_HEIGHT_COMPONENTS * width];

                strip::src_over(target, &color, alphas);
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

    use crate::fine::{COLOR_COMPONENTS, GradientIter, TILE_HEIGHT_COMPONENTS};
    use crate::util::scalar::div_255;
    use vello_common::paint::LinearGradient;

    pub(crate) fn src_over<T: Iterator<Item = [u8; COLOR_COMPONENTS]>>(
        target: &mut [u8],
        mut color_iter: T,
    ) {
        for strip in target.chunks_exact_mut(COLOR_COMPONENTS) {
            for bg_c in strip
                .chunks_exact_mut(COLOR_COMPONENTS)
            {
                let src_c = color_iter.next().unwrap();
                
                for i in 0..COLOR_COMPONENTS {
                    bg_c[i] = src_c[i] + div_255(bg_c[i] as u16 * (255 - src_c[3] as u16)) as u8;
                }
            }
        }
    }

    pub(crate) fn src_over_grad(target: &mut [u8], x: u16, grad: &LinearGradient) {
        let mut iter = GradientIter::new(grad, x);

        for strip in target.chunks_exact_mut(TILE_HEIGHT_COMPONENTS) {
            let src_c = iter.next().unwrap();
            let src_a = src_c[3] as u16;

            for bg_c in strip.chunks_exact_mut(COLOR_COMPONENTS) {
                for i in 0..COLOR_COMPONENTS {
                    bg_c[i] = src_c[i] + div_255(bg_c[i] as u16 * (255 - src_a)) as u8;
                }
            }
        }
    }
}

pub(crate) mod strip {
    use crate::fine::{COLOR_COMPONENTS, TILE_HEIGHT_COMPONENTS};
    use crate::util::scalar::div_255;
    use vello_common::tile::Tile;

    pub(crate) fn src_over(target: &mut [u8], src_c: &[u8; COLOR_COMPONENTS], alphas: &[u32]) {
        let src_a = src_c[3] as u16;

        for (bg_c, masks) in target.chunks_exact_mut(TILE_HEIGHT_COMPONENTS).zip(alphas) {
            for j in 0..usize::from(Tile::HEIGHT) {
                let mask_a = ((*masks >> (j * 8)) & 0xff) as u16;
                let inv_src_a_mask_a = 255 - div_255(mask_a * src_a);

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

pub struct GradientIter<'a> {
    cur_x: u16,
    c0: [u8; 4],
    c1: [u8; 4],
    colors: [u8; TILE_HEIGHT_COMPONENTS],
    gradient: &'a LinearGradient,
}

impl<'a> GradientIter<'a> {
    pub(crate) fn new(gradient: &'a LinearGradient, start_x: u16) -> Self {
        let c0 = gradient.stops[0].color.premultiply().to_rgba8_fast();
        let c1 = gradient.stops[1].color.premultiply().to_rgba8_fast();

        Self {
            cur_x: start_x,
            c0,
            c1,
            colors: [0; TILE_HEIGHT_COMPONENTS],
            gradient,
        }
    }
}

impl Iterator for GradientIter<'_> {
    type Item = [u8; TILE_HEIGHT_COMPONENTS];

    fn next(&mut self) -> Option<Self::Item> {
        let x0 = self.gradient.x1;
        let x1 = self.gradient.x2;

        for pix_idx in 0..Tile::HEIGHT as usize {
            for col_idx in 0..COLOR_COMPONENTS {
                let idx = pix_idx * COLOR_COMPONENTS + col_idx;
                let im1 = self.c1[col_idx] as f32 - self.c0[col_idx] as f32;
                let im2 = x1 - x0;
                let im3 = self.cur_x as f32 - x0;
                let combined = ((im1 / im2) * im3 + 0.5) as u8;

                self.colors[idx] = self.c0[col_idx] + combined
            }
        }

        self.cur_x += 1;

        Some(self.colors)
    }
}

fn splat_x4(val: &[u8; COLOR_COMPONENTS]) -> [u8; TILE_HEIGHT_COMPONENTS] {
    let mut buf = [0; TILE_HEIGHT_COMPONENTS];

    for i in 0..Tile::HEIGHT as usize {
        let start = i * COLOR_COMPONENTS;
        let end = (i + 1) * COLOR_COMPONENTS;
        buf[start..end].copy_from_slice(val);
    }

    buf
}

#[cfg(test)]
mod tests {
    use crate::fine::GradientIter;
    use vello_common::color::palette::css::{BLACK, BLUE, GREEN};
    use vello_common::paint::{LinearGradient, Stop};

    #[test]
    fn gradient_iter_1() {
        let gradient = LinearGradient {
            x1: 10.0,
            x2: 15.0,
            stops: vec![
                Stop {
                    offset: 0.0,
                    color: GREEN,
                },
                Stop {
                    offset: 1.0,
                    color: BLUE,
                },
            ],
        };

        let mut iter = GradientIter::new(&gradient, 10);
    }
}
