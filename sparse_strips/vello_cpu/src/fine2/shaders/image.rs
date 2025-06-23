use vello_common::encode::EncodedImage;
use vello_common::fearless_simd::{f32x4, u8x16, Simd};
use crate::fine2::PosExt;
use crate::fine2::shaders::image_simple::{extend_simd, sample, ImageFillerData};
use crate::peniko::ImageQuality;

#[derive(Debug)]
pub(crate) struct ImageFiller<'a, S: Simd> {
    data: ImageFillerData<'a, S>,
    simd: S,
}

impl<'a, S: Simd> ImageFiller<'a, S> {
    pub(crate) fn new(simd: S, image: &'a EncodedImage, start_x: u16, start_y: u16) -> Self {
        let data = ImageFillerData::new(simd, image, start_x, start_y);


        Self {
            data,
            simd,
        }
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