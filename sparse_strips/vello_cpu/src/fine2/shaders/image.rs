use crate::fine2::{u8_to_f32, PosExt};
use crate::kurbo::Point;
use vello_common::encode::EncodedImage;
use vello_common::fearless_simd::{Bytes, Simd, SimdBase, f32x4, u8x16, u32x4, f32x16, SimdFloat};
use crate::fine2::highp::element_wise_splat;
use crate::peniko::ImageQuality;

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

        let samples = sample(self.simd, &self.data, x_pos, self.y_positions);

        self.data.cur_pos += self.data.image.x_advance;

        Some(samples)
    }
}

#[derive(Debug)]
pub(crate) struct ImageFiller<'a, S: Simd> {
    data: ImageFillerData<'a, S>,
    simd: S,
}

impl<'a, S: Simd> ImageFiller<'a, S> {
    pub(crate) fn new(simd: S, image: &'a EncodedImage, start_x: u16, start_y: u16) -> Self {
        let data = ImageFillerData::new(simd, image, start_x, start_y);

        Self { data, simd }
    }
}

impl<S: Simd> Iterator for ImageFiller<'_, S> {
    type Item = u8x16<S>;

    fn next(&mut self) -> Option<Self::Item> {
        let x_positions = extend_simd(
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

        let y_positions = extend_simd(
            self.simd,
            f32x4::splat_col_pos(
                self.simd,
                self.data.cur_pos.y as f32,
                self.data.x_advances.1,
                self.data.y_advances.1,
            ),
            self.data.image.extends.1,
            self.data.height,
            self.data.height_inv,
        );

        let samples = sample(self.simd, &self.data, x_positions, y_positions);

        self.data.cur_pos += self.data.image.x_advance;

        Some(samples)
    }
}


#[derive(Debug)]
pub(crate) struct FilteredImageFiller<'a, S: Simd> {
    data: ImageFillerData<'a, S>,
    simd: S,
}

impl<'a, S: Simd> FilteredImageFiller<'a, S> {
    pub(crate) fn new(simd: S, image: &'a EncodedImage, start_x: u16, start_y: u16) -> Self {
        let data = ImageFillerData::new(simd, image, start_x, start_y);

        Self { data, simd }
    }
}

impl<S: Simd> Iterator for FilteredImageFiller<'_, S> {
    type Item = f32x16<S>;

    fn next(&mut self) -> Option<Self::Item> {
        let x_positions = extend_simd(
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

        let y_positions = extend_simd(
            self.simd,
            f32x4::splat_col_pos(
                self.simd,
                self.data.cur_pos.y as f32,
                self.data.x_advances.1,
                self.data.y_advances.1,
            ),
            self.data.image.extends.1,
            self.data.height,
            self.data.height_inv,
        );

        // We have two versions of filtering: `Medium` (bilinear filtering) and
        // `High` (bicubic filtering).

        // In bilinear filtering, we sample the pixels of the rectangle that spans the
        // locations (-0.5, -0.5) and (0.5, 0.5), and weight them by the fractional
        // x/y position using simple linear interpolation in both dimensions.
        // In bicubic filtering, we instead span a 4x4 grid around the
        // center of the location we are sampling, and sample those points
        // using a cubic filter to weight each location's contribution.

        let x_fract = element_wise_splat(self.simd, (x_positions + 0.5).fract());
        let y_fract = element_wise_splat(self.simd, (y_positions + 0.5).fract());

        let mut interpolated_color = f32x16::splat(self.simd, 0.0);

        let sample = |x_pos: f32x4<S>, y_pos: f32x4<S>| {
            u8_to_f32(sample(self.simd, &self.data, x_pos, y_pos)) * f32x16::splat(self.simd, 1.0 / 255.0)
        };

        match self.data.image.quality {
            ImageQuality::Low => unreachable!(),
            ImageQuality::Medium => {
                // <https://github.com/google/skia/blob/220738774f7a0ce4a6c7bd17519a336e5e5dea5b/src/opts/SkRasterPipeline_opts.h#L5039-L5078>
                let cx = [1.0 - x_fract, x_fract];
                let cy = [1.0 - y_fract, y_fract];

                // Note that the sum of all cx*cy combinations also yields 1.0 again
                // (modulo some floating point number impreciseness), ensuring the
                // colors stay in range.
                
                const OFFSETS: [f32; 2] = [-0.5, 0.5];
                
                let x_positions = [
                    extend_simd(
                        self.simd,
                        x_positions + OFFSETS[0],
                        self.data.image.extends.0,
                        self.data.width,
                        self.data.width_inv,
                    ),
                    extend_simd(
                        self.simd,
                        x_positions + OFFSETS[1],
                        self.data.image.extends.0,
                        self.data.width,
                        self.data.width_inv,
                    ),
                ];

                let y_positions = [
                    extend_simd(
                        self.simd,
                        y_positions + OFFSETS[0],
                        self.data.image.extends.0,
                        self.data.height,
                        self.data.height_inv,
                    ),
                    extend_simd(
                        self.simd,
                        y_positions + OFFSETS[1],
                        self.data.image.extends.0,
                        self.data.height,
                        self.data.height_inv,
                    ),
                ];

                // We sample the corners rectangle that covers our current position.
                for x_idx in 0..2 {
                    let x_positions = x_positions[x_idx];
                    
                    for y_idx in 0..2 {
                        let y_positions = y_positions[y_idx];
                        let color_sample = sample(x_positions, y_positions);
                        let w = cx[x_idx] * cy[y_idx];
                        
                        interpolated_color = interpolated_color.madd(w, color_sample);
                    }
                }
            }
            ImageQuality::High => unimplemented!()
        }
        
        // TODO: CLamp

        self.data.cur_pos += self.data.image.x_advance;

        Some(interpolated_color)
    }
}

#[derive(Debug)]
pub(crate) struct ImageFillerData<'a, S: Simd> {
    pub(crate) cur_pos: Point,
    pub(crate) image: &'a EncodedImage,
    pub(crate) x_advances: (f32, f32),
    pub(crate) y_advances: (f32, f32),
    pub(crate) height: f32x4<S>,
    pub(crate) height_inv: f32x4<S>,
    pub(crate) width: f32x4<S>,
    pub(crate) width_inv: f32x4<S>,
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

#[inline(always)]
pub(crate) fn sample<S: Simd>(
    simd: S,
    data: &ImageFillerData<S>,
    x_positions: f32x4<S>,
    y_positions: f32x4<S>,
) -> u8x16<S> {
    macro_rules! sample {
        ($idx:expr) => {
            data.image
                .pixmap
                .sample(x_positions.val[$idx] as u16, y_positions.val[$idx] as u16)
                .to_u32()
        };
    }

    u32x4::from_slice(simd, &[sample!(0), sample!(1), sample!(2), sample!(3)]).reinterpret_u8()
}

#[inline(always)]
pub(crate) fn extend_simd<S: Simd>(
    simd: S,
    val: f32x4<S>,
    extend: crate::peniko::Extend,
    max: f32x4<S>,
    inv_max: f32x4<S>,
) -> f32x4<S> {
    let bias = f32x4::splat(simd, 0.01);

    match extend {
        crate::peniko::Extend::Pad => val.min(max - bias).max(f32x4::splat(simd, 0.0)),
        crate::peniko::Extend::Repeat => val.msub((val * inv_max).floor(), max),
        // <https://github.com/google/skia/blob/220738774f7a0ce4a6c7bd17519a336e5e5dea5b/src/opts/SkRasterPipeline_opts.h#L3274-L3290>
        crate::peniko::Extend::Reflect => {
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
