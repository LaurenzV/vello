use smallvec::{smallvec, SmallVec};
use vello_common::encode::{EncodedGradient, GradientLike, GradientRange};
use vello_common::kurbo::Point;
use vello_simd::{Float, Type};
use crate::fine2::{Painter, TILE_HEIGHT_COMPONENTS};

#[derive(Debug)]
pub(crate) struct GradientFiller<'a, G: GradientLike> {
    cur_pos: Point,
    range_idx: usize,
    gradient: &'a EncodedGradient,
    kind: &'a G,
    cur_range: &'a GradientRange,
}

impl<'a, G: GradientLike> GradientFiller<'a, G> {
    pub(crate) fn new(
        gradient: &'a EncodedGradient,
        kind: &'a G,
        start_x: u16,
        start_y: u16,
    ) -> Self {
        Self {
            cur_pos: gradient.transform * Point::new(f64::from(start_x), f64::from(start_y)),
            range_idx: 0,
            cur_range: &gradient.ranges[0],
            gradient,
            kind,
        }
    }

    fn advance(&mut self, target_pos: f32) {
        while target_pos > self.cur_range.x1 || target_pos < self.cur_range.x0 {
            if self.range_idx == 0 {
                self.range_idx = self.gradient.ranges.len() - 1;
            } else {
                self.range_idx -= 1;
            }

            self.cur_range = &self.gradient.ranges[self.range_idx];
        }
    }

    pub(super) fn run<T: Type>(mut self, target: &mut [T::Scalar]) {
        let mut storage = vec![0.0f32; 16];
        
        target
            .chunks_exact_mut(TILE_HEIGHT_COMPONENTS)
            .for_each(|column| {
                self.run_column::<T>(column, &mut storage);
                self.cur_pos += self.gradient.x_advance;
            });
    }

    fn run_column<T: Type>(&mut self, col: &mut [T::Scalar], storage: &mut [f32]) {
        let pad = self.gradient.pad;
        let mut pos = self.cur_pos;
        
        for pixel in storage.chunks_exact_mut(4) {
            let res = self.single::<T::Float>(&pos, pad);
            res.store(pixel);

            pos += self.gradient.y_advance;
        }

        T::load_f32_many(storage).store(col);
    }
    
    #[inline(always)]
    fn single<T: Float>(&mut self, pos: &Point, pad: bool) -> T {
        let dist = extend(self.kind.cur_pos(*pos), pad);
        self.advance(dist);

        let dist = T::splat(dist);
        let range = self.cur_range;
        let x0 = T::splat(range.x0);
        let factors = T::load(&range.factors_f32);
        let c0 = T::load(&range.c0.as_premul_f32().components);

        let factor = factors * (dist - x0);
        c0 + factor
    } 

}

impl<F: Type, T: GradientLike> Painter<F> for GradientFiller<'_, T> {
    fn paint(self, target: &mut [F::Scalar]) {
        self.run::<F>(target);
    }
}

pub(crate) fn extend(mut val: f32, pad: bool) -> f32 {
    if pad {
        val
    } else {
        while val < 0.0 {
            val += 1.0;
        }

        while val > 1.0 {
            val -= 1.0;
        }

        val
    }
}
