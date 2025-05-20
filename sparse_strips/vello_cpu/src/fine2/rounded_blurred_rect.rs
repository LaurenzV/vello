// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Drawing blurred, rounded rectangles.
//!
//! Implementation is adapted from: <https://git.sr.ht/~raph/blurrr/tree/master/src/distfield.rs>.

use crate::fine2::{COLOR_COMPONENTS, Painter, TILE_HEIGHT_COMPONENTS};
use vello_common::encode::EncodedBlurredRoundedRectangle;
use vello_common::kurbo::Point;
use vello_common::tile::Tile;
use vello_simd::{Float, Type};

#[derive(Debug)]
pub(crate) struct BlurredRoundedRectFiller<'a> {
    /// The current position that should be processed.
    cur_pos: Point,
    /// The underlying encoded blurred rectangle.
    rect: &'a EncodedBlurredRoundedRectangle,
}

impl<'a> BlurredRoundedRectFiller<'a> {
    pub(crate) fn new(
        rect: &'a EncodedBlurredRoundedRectangle,
        start_x: u16,
        start_y: u16,
    ) -> Self {
        Self {
            cur_pos: rect.transform * Point::new(f64::from(start_x), f64::from(start_y)),
            rect,
        }
    }

    pub(super) fn run<F: Type>(mut self, target: &mut [F::Scalar]) {
        let h = F::Float::splat(self.rect.h);
        let w = F::Float::splat(self.rect.w);
        let width = F::Float::splat(self.rect.width);
        let height = F::Float::splat(self.rect.height);
        let r1 = F::Float::splat(self.rect.r1);
        let exponent = self.rect.exponent;
        let recip_exponent = self.rect.recip_exponent;
        let scale = F::Float::splat(self.rect.scale);
        let min_edge = F::Float::splat(self.rect.min_edge);
        let std_dev_inv = F::Float::splat(self.rect.std_dev_inv);
        let start_pos = self.cur_pos;

        let mut cur_pos = self.cur_pos;

        let calc_pos = |idx: usize| {
            let col_idx = idx / (COLOR_COMPONENTS * Tile::HEIGHT as usize);
            let row_idx = idx & (COLOR_COMPONENTS * Tile::HEIGHT as usize - 1);

            start_pos + self.rect.x_advance * col_idx as f64 + self.rect.y_advance * row_idx as f64
        };

        let color = F::splat_color(self.rect.color);
        let mut storage = vec![];
        let mut idx = 0;

        for column in target.chunks_exact_mut(F::LENGTH) {
            storage.truncate(0);

            for _ in 0..(F::LENGTH / F::Float::LENGTH) {
                let (i, j) = F::Float::splat_col_pos(
                    (cur_pos.x as f32, cur_pos.y as f32),
                    (self.rect.x_advance.x as f32, self.rect.x_advance.y as f32),
                    (self.rect.y_advance.x as f32, self.rect.y_advance.y as f32),
                );

                let alpha_val = {
                    let v0 = F::Float::splat(0.0);
                    let v1 = F::Float::splat(0.5);

                    let y = j + v1 - v1 * height;
                    let y0 = y.abs() - (h * v1 - r1);
                    let y1 = y0.max(v0);

                    let x = i + v1 - v1 * width;
                    let x0 = x.abs() - (w * v1 - r1);
                    let x1 = x0.max(v0);
                    let d_pos = (x1.powf(exponent) + y1.powf(exponent)).powf(recip_exponent);
                    let d_neg = x0.max(y0).min(v0);
                    let d = d_pos + d_neg - r1;
                    let z = scale
                        * (F::Float::compute_erf7(std_dev_inv * (min_edge + d))
                            - F::Float::compute_erf7(std_dev_inv * d));

                    z
                };

                storage.push(alpha_val);

                idx += F::Float::LENGTH;

                cur_pos = calc_pos(idx);
            }

            let loaded_alpha = F::from_float(&storage);
            let multiplied = color.normalized_mul(loaded_alpha);

            multiplied.store(column);
        }
    }
}

impl<F: Type> Painter<F> for BlurredRoundedRectFiller<'_> {
    fn paint(self, target: &mut [F::Scalar]) {
        self.run::<F>(target);
    }
}
