// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Drawing blurred, rounded rectangles.
//!
//! Implementation is adapted from: <https://git.sr.ht/~raph/blurrr/tree/master/src/distfield.rs>.

use crate::fine2::PosExt;
use crate::fine2::highp::{element_wise_splat};
use crate::kurbo::{Point, Vec2};
use vello_common::encode::EncodedBlurredRoundedRectangle;
use vello_common::fearless_simd::{Simd, SimdBase, SimdFloat, SimdInto, f32x4, f32x8, f32x16};

#[derive(Debug)]
pub(crate) struct BlurredRoundedRectFiller<S: Simd> {
    color: f32x16<S>,
    alpha_calculator: AlphaCalculator<S>,
    next_alphas: Option<f32x4<S>>,
    simd: S,
}

impl<S: Simd> BlurredRoundedRectFiller<S> {
    pub(crate) fn new(
        simd: S,
        rect: &EncodedBlurredRoundedRectangle,
        start_x: u16,
        start_y: u16,
    ) -> Self {
        let start_pos = rect.transform * Point::new(f64::from(start_x), f64::from(start_y));
        let color = f32x16::block_splat(rect.color.as_premul_f32().components.simd_into(simd));
        let simd_rect = SimdRoundedBlurredRect::new(rect, simd);
        let alpha_calculator =
            AlphaCalculator::new(start_pos, rect.x_advance, rect.y_advance, simd_rect, simd);

        Self {
            alpha_calculator,
            color,
            simd,
            next_alphas: None,
        }
    }
}

impl<S: Simd> Iterator for BlurredRoundedRectFiller<S> {
    type Item = f32x16<S>;

    fn next(&mut self) -> Option<Self::Item> {
        let alphas = if let Some(next) = self.next_alphas {
            self.next_alphas = None;

            element_wise_splat(self.simd, next)
        } else {
            let next = self.alpha_calculator.next().unwrap();
            let (s0, s1) = self.simd.split_f32x8(next);
            self.next_alphas = Some(s1);

            element_wise_splat(self.simd, s0)
        };

        Some(self.color * alphas)
    }
}

#[derive(Debug)]
struct AlphaCalculator<S: Simd> {
    cur_pos: Point,
    x_advance: Vec2,
    y_advance: Vec2,
    r: SimdRoundedBlurredRect<S>,
    simd: S,
}

impl<'a, S: Simd> AlphaCalculator<S> {
    fn new(
        start_pos: Point,
        x_advance: Vec2,
        y_advance: Vec2,
        r: SimdRoundedBlurredRect<S>,
        simd: S,
    ) -> Self {
        Self {
            cur_pos: start_pos,
            x_advance,
            y_advance,
            r,
            simd,
        }
    }
}

impl<S: Simd> Iterator for AlphaCalculator<S> {
    type Item = f32x8<S>;

    fn next(&mut self) -> Option<Self::Item> {
        let i = f32x8::splat_col_pos(
            self.simd,
            self.cur_pos.x as f32,
            self.x_advance.x as f32,
            self.y_advance.x as f32,
        );
        let j = f32x8::splat_col_pos(
            self.simd,
            self.cur_pos.y as f32,
            self.x_advance.y as f32,
            self.y_advance.y as f32,
        );
        let r = &self.r;

        let y = j + r.v1.msub(r.v1, r.height);
        let y0 = y.abs().msub(r.v1, r.h) + r.r1;
        let y1 = y0.max(r.v0);

        let x = i + r.v1.msub(r.v1, r.width);
        let x0 = x.abs().msub(r.v1, r.w) + r.r1;
        let x1 = x0.max(r.v0);
        let d_pos = (x1.powf(r.exponent) + y1.powf(r.exponent)).powf(r.recip_exponent);
        let d_neg = x0.max(y0).min(r.v0);
        let d = d_pos + d_neg - r.r1;
        let z = r.scale
            * (f32x8::compute_erf7(self.simd, r.std_dev_inv * (r.min_edge + d))
                - f32x8::compute_erf7(self.simd, r.std_dev_inv * d));

        self.cur_pos += 2.0 * self.x_advance;

        Some(z)
    }
}

#[derive(Debug)]
struct SimdRoundedBlurredRect<S: Simd> {
    pub exponent: f32,
    pub recip_exponent: f32,
    pub scale: f32x8<S>,
    pub std_dev_inv: f32x8<S>,
    pub min_edge: f32x8<S>,
    pub w: f32x8<S>,
    pub h: f32x8<S>,
    pub width: f32x8<S>,
    pub height: f32x8<S>,
    pub r1: f32x8<S>,
    pub v0: f32x8<S>,
    pub v1: f32x8<S>,
}

impl<S: Simd> SimdRoundedBlurredRect<S> {
    fn new(encoded: &EncodedBlurredRoundedRectangle, s: S) -> Self {
        let h = f32x8::splat(s, encoded.h);
        let w = f32x8::splat(s, encoded.w);
        let width = f32x8::splat(s, encoded.width);
        let height = f32x8::splat(s, encoded.height);
        let r1 = f32x8::splat(s, encoded.r1);
        let exponent = encoded.exponent;
        let recip_exponent = encoded.recip_exponent;
        let scale = f32x8::splat(s, encoded.scale);
        let min_edge = f32x8::splat(s, encoded.min_edge);
        let std_dev_inv = f32x8::splat(s, encoded.std_dev_inv);
        let v0 = f32x8::splat(s, 0.0);
        let v1 = f32x8::splat(s, 0.5);

        Self {
            exponent,
            recip_exponent,
            scale,
            std_dev_inv,
            min_edge,
            w,
            v0,
            v1,
            h,
            width,
            height,
            r1,
        }
    }
}

trait FloatExt<S: Simd> {
    // See https://raphlinus.github.io/audio/2018/09/05/sigmoid.html for a little
    // explanation of this approximation to the erf function.
    // Doing `inline(always)` seems to reduce performance for some reason.
    /// Approximate the erf function.
    fn compute_erf7(simd: S, x: Self) -> Self;
    fn powf(self, x: f32) -> Self;
}

impl<S: Simd> FloatExt<S> for f32x8<S> {
    fn compute_erf7(simd: S, x: Self) -> Self {
        let x = x * f32x8::splat(simd, core::f32::consts::FRAC_2_SQRT_PI);
        let xx = x * x;
        let p1 = f32x8::splat(simd, 0.03395).madd(f32x8::splat(simd, 0.0104), xx);
        let p2 = f32x8::splat(simd, 0.24295).madd(p1, xx);
        let p3 = x * xx;
        let x = x.madd(p2, p3);
        let denom = f32x8::splat(simd, 1.0).madd(x, x).sqrt();
        x / denom
    }

    #[inline]
    fn powf(mut self, x: f32) -> Self {
        // TODO: SIMD
        self.val[0] = self.val[0].powf(x);
        self.val[1] = self.val[1].powf(x);
        self.val[2] = self.val[2].powf(x);
        self.val[3] = self.val[3].powf(x);
        self.val[4] = self.val[4].powf(x);
        self.val[5] = self.val[5].powf(x);
        self.val[6] = self.val[6].powf(x);
        self.val[7] = self.val[7].powf(x);

        self
    }
}
