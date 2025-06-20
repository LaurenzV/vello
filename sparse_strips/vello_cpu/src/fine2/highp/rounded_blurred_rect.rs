// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Drawing blurred, rounded rectangles.
//!
//! Implementation is adapted from: <https://git.sr.ht/~raph/blurrr/tree/master/src/distfield.rs>.

use vello_common::encode::EncodedBlurredRoundedRectangle;
use vello_common::fearless_simd::{f32x8, Simd, SimdBase, SimdFloat};

// #[derive(Debug)]
// pub(crate) struct BlurredRoundedRectFiller<T: Type> {
//     /// The underlying encoded blurred rectangle.
//     rect: SimdRectangle<T::Float>,
//     start_pos: Point,
//     x_advance: Vec2,
//     y_advance: Vec2,
//     color: T,
// }
// 
// impl<T: Type> BlurredRoundedRectFiller<T> {
//     pub(crate) fn new(rect: &EncodedBlurredRoundedRectangle, start_x: u16, start_y: u16) -> Self {
//         let start_pos = rect.transform * Point::new(f64::from(start_x), f64::from(start_y));
//         let x_advance = rect.x_advance;
//         let y_advance = rect.y_advance;
//         let color = T::splat_color(rect.color);
//         let rect = SimdRectangle::<T::Float>::new(rect);
// 
//         Self {
//             start_pos,
//             rect,
//             x_advance,
//             y_advance,
//             color,
//         }
//     }
// 
//     pub(super) fn run(mut self, target: &mut [T::Scalar]) {
//         let mut alpha_calculator = AlphaCalculator::<T::Float>::new(
//             self.start_pos,
//             self.x_advance,
//             self.y_advance,
//             &self.rect,
//         );
//         let color = self.color;
// 
//         if T::LENGTH / 4 >= T::Float::LENGTH {
//             let mut storage = vec![];
//             for column in target.chunks_exact_mut(T::LENGTH) {
//                 storage.clear();
// 
//                 for _ in 0..((T::LENGTH / 4) / T::Float::LENGTH) {
//                     storage.push(alpha_calculator.next().unwrap());
//                 }
// 
//                 let loaded = T::from_float(storage.as_slice());
//                 let mulled = loaded.normalized_mul(color);
// 
//                 mulled.store(column);
//             }
//         } else {
//             let mut iter = target.chunks_exact_mut(T::LENGTH);
// 
//             'outer: loop {
//                 let mut stored_alpha = vec![0.0f32; T::Float::LENGTH];
//                 let alphas = alpha_calculator.next().unwrap();
//                 alphas.store(&mut stored_alpha);
// 
//                 for alphas in stored_alpha.chunks_exact(T::LENGTH / 4) {
//                     let Some(column) = iter.next() else {
//                         break 'outer;
//                     };
// 
//                     let t = T::load_alphas_f32(alphas);
//                     let mulled = t.normalized_mul(color);
//                     mulled.store(column);
//                 }
//             }
//         }
//     }
// }
// 
// struct AlphaCalculator<'a, F: Float> {
//     start_pos: Point,
//     x_advance: Vec2,
//     y_advance: Vec2,
//     r: &'a SimdRectangle<F>,
//     idx: usize,
// }
// 
// impl<'a, F: Float> AlphaCalculator<'a, F> {
//     fn new(start_pos: Point, x_advance: Vec2, y_advance: Vec2, r: &'a SimdRectangle<F>) -> Self {
//         Self {
//             start_pos,
//             x_advance,
//             y_advance,
//             r,
//             idx: 0,
//         }
//     }
// }
// 
// impl<F: Float> Iterator for AlphaCalculator<'_, F> {
//     type Item = F;
// 
//     fn next(&mut self) -> Option<Self::Item> {
//         let calc_pos = |idx: usize| {
//             let col_idx = idx >> (Tile::HEIGHT.trailing_zeros() as usize);
//             let row_idx = idx & (Tile::HEIGHT as usize - 1);
// 
//             self.start_pos + self.x_advance * col_idx as f64 + self.y_advance * row_idx as f64
//         };
// 
//         let pos = calc_pos(self.idx);
// 
//         let i = F::Float::splat_x_col_pos(pos.x as f32, self.x_advance.x as f32, self.y_advance.x as f32);
//         let j = F::Float::splat_y_col_pos(pos.y as f32, self.x_advance.y as f32, self.y_advance.y as f32);
//         let r = self.r;
// 
//         let y = j + r.height.mul_sub(r.v1, r.v1);
//         let y0 = r.h.mul_sub(r.v1, y.abs()) + r.r1;
//         let y1 = y0.max(r.v0);
// 
//         let x = i + r.width.mul_sub(r.v1, r.v1);
//         let x0 = r.w.mul_sub(r.v1, x.abs()) + r.r1;
//         let x1 = x0.max(r.v0);
//         let d_pos = (x1.powf(r.exponent) + y1.powf(r.exponent)).powf(r.recip_exponent);
//         let d_neg = x0.max(y0).min(r.v0);
//         let d = d_pos + d_neg - r.r1;
//         let z = r.scale
//             * (F::compute_erf7(r.std_dev_inv * (r.min_edge + d))
//                 - F::compute_erf7(r.std_dev_inv * d));
// 
//         self.idx += F::LENGTH;
// 
//         Some(z)
//     }
// }

#[derive(Debug)]
struct SimdRectangle<S: Simd> {
    pub exponent: f32,
    pub recip_exponent: f32,
    pub scale: f32x8<S>,
    pub std_dev_inv: f32x8<S>,
    pub simd: S,
    pub min_edge: f32x8<S>,
    pub w: f32x8<S>,
    pub h: f32x8<S>,
    pub width: f32x8<S>,
    pub height: f32x8<S>,
    pub r1: f32x8<S>,
    pub v0: f32x8<S>,
    pub v1: f32x8<S>,
}

impl<S: Simd> SimdRectangle<S> {
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
            simd: s
        }
    }
}
// 
// impl<F: Type> Painter<F> for BlurredRoundedRectFiller<F> {
//     fn paint(self, target: &mut [F::Scalar]) {
//         self.run(target);
//     }
// }

trait FloatExt {
    // See https://raphlinus.github.io/audio/2018/09/05/sigmoid.html for a little
    // explanation of this approximation to the erf function.
    // Doing `inline(always)` seems to reduce performance for some reason.
    /// Approximate the erf function.
    fn compute_erf7(self) -> Self;
}

impl<S: Simd> FloatExt for f32x8<S> {
    fn compute_erf7(self) -> Self {
        let x = self * f32x8::splat(self.simd, core::f32::consts::FRAC_2_SQRT_PI);
        let xx = x * x;
        let p1 = xx.madd(f32x8::splat(self.simd, 0.0104), f32x8::splat(self.simd, 0.03395));
        let p2 = xx.madd(p1, f32x8::splat(self.simd, 0.24295));
        let p3 = x * xx;
        let x = p3.madd(p2, x);
        let denom = x.madd(x, f32x8::splat(self.simd, 1.0)).sqrt();
        x / denom
    }
} 