// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::fine2::{COLOR_COMPONENTS, PosExt, TILE_HEIGHT_COMPONENTS};
use vello_common::encode::EncodedImage;
use vello_common::fearless_simd::{Bytes, Simd, SimdBase, f32x4, f32x16, u8x16, u32x4};
use vello_common::kurbo::{Point, Vec2};
use vello_common::peniko::{Extend, ImageQuality};

#[derive(Debug)]
pub(crate) struct ImageFillerData<'a, S: Simd> {
    cur_pos: Point,
    image: &'a EncodedImage,
    x_advances: (f32, f32),
    y_advances: (f32, f32),
    height: f32x4<S>,
    height_inv: f32x4<S>,
    width: f32x4<S>,
    width_inv: f32x4<S>,
}

impl<'a, S: Simd> ImageFillerData<'a, S> {
    pub(crate) fn new(simd: S, image: &'a EncodedImage, start_x: u16, start_y: u16) -> Self {
        let width = image.pixmap.width() as f32;
        let height = image.pixmap.height() as f32;
        let start_pos = image.transform * Point::new(f64::from(start_x), f64::from(start_y));

        let width_inv = f32x4::splat(simd, 1.0 / width);
        let height_inv = f32x4::splat(simd, 1.0 / height);
        let width = f32x4::splat(simd, width);
        let height = f32x4::splat(simd, height);

        let x_advances = (image.x_advance.x as f32, image.x_advance.y as f32);
        let y_advances = (image.y_advance.x as f32, image.y_advance.y as f32);

        Self {
            cur_pos: start_pos,
            x_advances,
            y_advances,
            image,
            width,
            height,
            width_inv,
            height_inv,
        }
    }
}

#[derive(Debug)]
pub(crate) struct SimpleImageFiller<'a, S: Simd> {
    data: ImageFillerData<'a, S>,
    y_positions: f32x4<S>,
    simd: S,
}

impl<'a, S: Simd> SimpleImageFiller<'a, S> {
    pub(crate) fn new(simd: S, image: &'a EncodedImage, start_x: u16, start_y: u16) -> Self {
        let data = ImageFillerData::new(simd, image, start_x, start_y);

        let y_positions = extend_simd(
            simd,
            f32x4::splat_col_pos(
                simd,
                data.cur_pos.y as f32,
                data.x_advances.1,
                data.y_advances.1,
            ),
            image.extends.1,
            data.height,
            data.height_inv,
        );

        Self {
            data,
            y_positions,
            simd,
        }
    }
}

impl<S: Simd> Iterator for SimpleImageFiller<'_, S> {
    type Item = u8x16<S>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let x_pos = extend_simd(
            self.simd,
            f32x4::splat_col_pos(
                self.simd,
                self.data.cur_pos.x as f32,
                self.data.x_advances.0,
                self.data.y_advances.0,
            ),
            self.data.image.extends.0,
            self.data.width,
            self.data.width_inv,
        );

        macro_rules! sample {
            ($idx:expr) => {
                self.data
                    .image
                    .pixmap
                    .sample(x_pos.val[$idx] as u16, self.y_positions.val[$idx] as u16)
                    .to_u32()
            };
        }

        let samples =
            u32x4::from_slice(self.simd, &[sample!(0), sample!(1), sample!(2), sample!(3)])
                .reinterpret_u8();

        self.data.cur_pos += self.data.image.x_advance;

        Some(samples)
    }
}

#[inline(always)]
fn extend_simd<S: Simd>(
    simd: S,
    val: f32x4<S>,
    extend: Extend,
    max: f32x4<S>,
    inv_max: f32x4<S>,
) -> f32x4<S> {
    let bias = f32x4::splat(simd, 0.01);

    match extend {
        Extend::Pad => val.min(max - bias).max(f32x4::splat(simd, 0.0)),
        Extend::Repeat => val - (val * inv_max).floor() * max,
        // <https://github.com/google/skia/blob/220738774f7a0ce4a6c7bd17519a336e5e5dea5b/src/opts/SkRasterPipeline_opts.h#L3274-L3290>
        Extend::Reflect => {
            let u = val
                - (val * inv_max * f32x4::splat(simd, 0.5)).floor() * f32x4::splat(simd, 2.0) * max;
            let s = (u * inv_max).floor();
            let m = u - f32x4::splat(simd, 2.0) * s * (u - max);

            let bias_in_ulps = s.trunc();

            let m_bits = u32x4::from_bytes(m.to_bytes());
            let biased_bits = m_bits.wrapping_sub(bias_in_ulps.cvt_u32());
            f32x4::from_bytes(biased_bits.to_bytes())
        }
    }
}
