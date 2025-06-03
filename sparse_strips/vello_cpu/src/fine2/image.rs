// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::fine2::{COLOR_COMPONENTS, Painter, TILE_HEIGHT_COMPONENTS};
use vello_common::encode::EncodedImage;
use vello_common::kurbo::{Point, Vec2};
use vello_common::peniko::{Extend, ImageQuality};
use vello_common::tile::Tile;
use vello_simd::{Float, Index, NumberKind, Type};

#[derive(Debug)]
pub(crate) struct ImageFiller<'a> {
    /// The current position that should be processed.
    cur_pos: Point,
    /// The underlying image.
    image: &'a EncodedImage,
    // Precomputed values reused in per-pixel calculations.
    x_advances: (f32, f32),
    y_advances: (f32, f32),
    height: f32,
    height_inv: f32,
    width: f32,
    width_inv: f32,
}

impl<'a> ImageFiller<'a> {
    pub(crate) fn new(image: &'a EncodedImage, start_x: u16, start_y: u16) -> Self {
        let width = image.pixmap.width() as f32;
        let height = image.pixmap.height() as f32;

        Self {
            cur_pos: image.transform * Point::new(f64::from(start_x), f64::from(start_y)),
            x_advances: (image.x_advance.x as f32, image.x_advance.y as f32),
            y_advances: (image.y_advance.x as f32, image.y_advance.y as f32),
            image,
            width,
            height,
            width_inv: 1.0 / width,
            height_inv: 1.0 / height,
        }
    }

    pub(super) fn run<F: Type>(mut self, target: &mut [F::Scalar]) {
        let mut x_pos_storage: [f32; 16] = [0.0; 16];
        let mut y_pos_storage: [f32; 16] = [0.0; 16];

        let y_pos = F::Float::splat_y_col_pos(self.cur_pos.y as f32, self.x_advances.1, self.y_advances.1);
        let extended_y_pos = extend_simd(
            y_pos,
            self.image.extends.1,
            F::Float::splat(self.height),
            F::Float::splat(self.height_inv),
        );
        extended_y_pos.store(&mut y_pos_storage);

        // We currently have two branches for filling images: The first case is used for
        // nearest neighbor filtering and for images with no skewing-transform (this is checked
        // by the first two conditions), which allows us to take a faster path.
        // The second version is the general case for any other image.
        // Once we get to performance optimizations, it's possible that there will be further
        // paths (e.g. one for no scaling transform and only integer translation offsets).
        if self.image.y_advance.x != 0.0
            || self.image.x_advance.y != 0.0
            || self.image.quality != ImageQuality::Low
        {
            // Fallback path.
            // target
            //     .chunks_exact_mut(TILE_HEIGHT_COMPONENTS)
            //     .for_each(|column| {
            //         self.run_complex_column(column);
            //         self.cur_pos += self.image.x_advance;
            //     });
        } else {
            target.chunks_exact_mut(64).for_each(|column| {
                let x_pos = F::Float::splat_x_col_pos(self.cur_pos.x as f32, self.x_advances.0, self.y_advances.0);
                
                let extended_x_pos = extend_simd(
                    x_pos,
                    self.image.extends.0,
                    F::Float::splat(self.width),
                    F::Float::splat(self.width_inv),
                );
                
                extended_x_pos.store(&mut x_pos_storage);

                self.run_simple_column::<F>(column, x_pos_storage, y_pos_storage);

                self.cur_pos += self.image.x_advance * 4.0;
            });
        }
    }

    // // Ideally, we'd add this only on the specific arm, but clippy doesn't support that
    // #[expect(
    //     clippy::trivially_copy_pass_by_ref,
    //     reason = "Tile::HEIGHT is expected to increase later."
    // )]
    fn run_simple_column<T: Type>(
        &mut self,
        col: &mut [T::Scalar],
        x_pos: [f32; 16],
        y_pos: [f32; 16],
    ) {
        for ((pixel, y_pos), x_pos) in col
            .chunks_exact_mut(COLOR_COMPONENTS)
            .zip(y_pos.iter())
            .zip(x_pos.iter())
        {
            let sample = self
                .image
                .pixmap
                .sample(*x_pos as u16, *y_pos as u16)
                .to_u8_array();

            pixel.copy_from_slice(&T::Scalar::from_rgba8(&sample));
        }
    }
    //
    // fn run_complex_column<F: FineType>(&mut self, col: &mut [F]) {
    //     let extend_point = |mut point: Point| {
    //         point.x = f64::from(extend(
    //             point.x as f32,
    //             self.image.extends.0,
    //             self.width,
    //             self.width_inv,
    //         ));
    //         point.y = f64::from(extend(
    //             point.y as f32,
    //             self.image.extends.1,
    //             self.height,
    //             self.height_inv,
    //         ));
    //
    //         point
    //     };
    //
    //     let mut pos = self.cur_pos;
    //
    //     for pixel in col.chunks_exact_mut(COLOR_COMPONENTS) {
    //         match self.image.quality {
    //             // Nearest neighbor filtering.
    //             // Simply takes the nearest pixel to our current position.
    //             ImageQuality::Low => {
    //                 let point = extend_point(pos);
    //                 let sample = F::from_rgba8(
    //                     &self
    //                         .image
    //                         .pixmap
    //                         .sample(point.x as u16, point.y as u16)
    //                         .to_u8_array()[..],
    //                 );
    //                 pixel.copy_from_slice(&sample);
    //             }
    //             ImageQuality::Medium | ImageQuality::High => {
    //                 // We have two versions of filtering: `Medium` (bilinear filtering) and
    //                 // `High` (bicubic filtering).
    //
    //                 // In bilinear filtering, we sample the pixels of the rectangle that spans the
    //                 // locations (-0.5, -0.5) and (0.5, 0.5), and weight them by the fractional
    //                 // x/y position using simple linear interpolation in both dimensions.
    //                 // In bicubic filtering, we instead span a 4x4 grid around the
    //                 // center of the location we are sampling, and sample those points
    //                 // using a cubic filter to weight each location's contribution.
    //
    //                 fn fract(val: f32) -> f32 {
    //                     val - floor(val)
    //                 }
    //
    //                 let x_fract = fract(pos.x as f32 + 0.5);
    //                 let y_fract = fract(pos.y as f32 + 0.5);
    //
    //                 let mut interpolated_color = [0.0_f32; 4];
    //
    //                 let sample = |p: Point| {
    //                     let c = |val: u8| f32::from(val) / 255.0;
    //                     let s = self.image.pixmap.sample(p.x as u16, p.y as u16);
    //
    //                     [c(s.r), c(s.g), c(s.b), c(s.a)]
    //                 };
    //
    //                 if self.image.quality == ImageQuality::Medium {
    //                     // <https://github.com/google/skia/blob/220738774f7a0ce4a6c7bd17519a336e5e5dea5b/src/opts/SkRasterPipeline_opts.h#L5039-L5078>
    //                     let cx = [1.0 - x_fract, x_fract];
    //                     let cy = [1.0 - y_fract, y_fract];
    //
    //                     // Note that the sum of all cx*cy combinations also yields 1.0 again
    //                     // (modulo some floating point number impreciseness), ensuring the
    //                     // colors stay in range.
    //
    //                     // We sample the corners rectangle that covers our current position.
    //                     for (x_idx, x) in [-0.5, 0.5].into_iter().enumerate() {
    //                         for (y_idx, y) in [-0.5, 0.5].into_iter().enumerate() {
    //                             let color_sample = sample(extend_point(pos + Vec2::new(x, y)));
    //                             let w = cx[x_idx] * cy[y_idx];
    //
    //                             for (component, component_sample) in
    //                                 interpolated_color.iter_mut().zip(color_sample)
    //                             {
    //                                 *component += w * component_sample;
    //                             }
    //                         }
    //                     }
    //                 } else {
    //                     // Compare to <https://github.com/google/skia/blob/84ff153b0093fc83f6c77cd10b025c06a12c5604/src/opts/SkRasterPipeline_opts.h#L5030-L5075>.
    //                     let cx = weights(x_fract);
    //                     let cy = weights(y_fract);
    //
    //                     // Note in particular that it is guaranteed that, similarly to bilinear filtering,
    //                     // the sum of all cx*cy is 1.
    //
    //                     // We sample the 4x4 grid around the position we are currently looking at.
    //                     for (x_idx, x) in [-1.5, -0.5, 0.5, 1.5].into_iter().enumerate() {
    //                         for (y_idx, y) in [-1.5, -0.5, 0.5, 1.5].into_iter().enumerate() {
    //                             let color_sample = sample(extend_point(pos + Vec2::new(x, y)));
    //                             let c = cx[x_idx] * cy[y_idx];
    //
    //                             for (component, component_sample) in
    //                                 interpolated_color.iter_mut().zip(color_sample)
    //                             {
    //                                 *component += c * component_sample;
    //                             }
    //                         }
    //                     }
    //                 }
    //
    //                 for i in 0..COLOR_COMPONENTS {
    //                     // Due to the nature of the cubic filter, it can happen in certain situations
    //                     // that one of the color components ends up with a higher value than the
    //                     // alpha component, which isn't permissible because the color is
    //                     // premultiplied and would lead to overflows when doing source over
    //                     // compositing with u8-based values. Because of this, we need to clamp
    //                     // to the alpha value.
    //                     let f32_val = interpolated_color[i]
    //                         .clamp(0.0, 1.0)
    //                         .min(interpolated_color[3]);
    //                     interpolated_color[i] = f32_val;
    //                 }
    //
    //                 pixel.copy_from_slice(&F::from_rgbaf32(&interpolated_color[..]));
    //             }
    //         };
    //
    //         pos += self.image.y_advance;
    //     }
    // }
}

#[inline(always)]
fn extend_simd<T: Float>(val: T, extend: Extend, max: T, inv_max: T) -> T {
    let bias = T::splat(0.01);

    match extend {
        Extend::Pad => val.min(max - bias).max(T::splat(0.0)),
        Extend::Repeat => val - (val * inv_max).floor() * max,
        // <https://github.com/google/skia/blob/220738774f7a0ce4a6c7bd17519a336e5e5dea5b/src/opts/SkRasterPipeline_opts.h#L3274-L3290>
        Extend::Reflect => {
            let u = val - (val * inv_max * T::splat(0.5)).floor() * T::splat(2.0) * max;
            let s = (u * inv_max).floor();
            let m = u - T::splat(2.0) * s * (u - max);

            let bias_in_ulps = s.trunc();

            let m_bits = m.reinterpret();
            let biased_bits = m_bits.wrapping_sub(bias_in_ulps.to_integer());
            biased_bits.reinterpret()
        }
    }
}

impl<F: Type> Painter<F> for ImageFiller<'_> {
    fn paint(self, target: &mut [F::Scalar]) {
        self.run::<F>(target);
    }
}

// /// Calculate the weights for a single fractional value.
// const fn weights(fract: f32) -> [f32; 4] {
//     const MF: [[f32; 4]; 4] = mf_resampler();
//
//     [
//         single_weight(fract, MF[0][0], MF[0][1], MF[0][2], MF[0][3]),
//         single_weight(fract, MF[1][0], MF[1][1], MF[1][2], MF[1][3]),
//         single_weight(fract, MF[2][0], MF[2][1], MF[2][2], MF[2][3]),
//         single_weight(fract, MF[3][0], MF[3][1], MF[3][2], MF[3][3]),
//     ]
// }
//
// /// Calculate a weight based on the fractional value t and the cubic coefficients.
// const fn single_weight(t: f32, a: f32, b: f32, c: f32, d: f32) -> f32 {
//     t * (t * (t * d + c) + b) + a
// }
//
// /// Mitchell filter with the variables B = 1/3 and C = 1/3.
// const fn mf_resampler() -> [[f32; 4]; 4] {
//     cubic_resampler(1.0 / 3.0, 1.0 / 3.0)
// }
//
// /// Cubic resampling logic is borrowed from Skia. See
// /// <https://github.com/google/skia/blob/220fef664978643a47d4559ae9e762b91aba534a/include/core/SkSamplingOptions.h#L33-L50>
// /// for some links to understand how this works. In principle, this macro allows us to define a
// /// resampler kernel based on two variables B and C which can be between 0 and 1, allowing to
// /// change some properties of the cubic interpolation kernel.
// ///
// /// As mentioned above, cubic resampling consists of sampling the 16 surrounding pixels of the
// /// target point and interpolating them with a cubic filter.
// /// The generated matrix is 4x4 and represent the coefficients of the cubic function used to
// /// calculate weights based on the `x_fract` and `y_fract` of the location we are looking at.
// const fn cubic_resampler(b: f32, c: f32) -> [[f32; 4]; 4] {
//     [
//         [
//             (1.0 / 6.0) * b,
//             -(3.0 / 6.0) * b - c,
//             (3.0 / 6.0) * b + 2.0 * c,
//             -(1.0 / 6.0) * b - c,
//         ],
//         [
//             1.0 - (2.0 / 6.0) * b,
//             0.0,
//             -3.0 + (12.0 / 6.0) * b + c,
//             2.0 - (9.0 / 6.0) * b - c,
//         ],
//         [
//             (1.0 / 6.0) * b,
//             (3.0 / 6.0) * b + c,
//             3.0 - (15.0 / 6.0) * b - 2.0 * c,
//             -2.0 + (9.0 / 6.0) * b + c,
//         ],
//         [0.0, 0.0, -c, (1.0 / 6.0) * b + c],
//     ]
// }
