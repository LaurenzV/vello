use crate::fine2::{COLOR_COMPONENTS, Painter, TILE_HEIGHT_COMPONENTS};
use smallvec::{SmallVec, smallvec};
use vello_common::encode::{EncodedGradient, GradientLike, GradientRange};
use vello_common::kurbo::Point;
use vello_common::tile::Tile;
use vello_simd::{Float, Type};

#[derive(Debug)]
pub(crate) struct GradientFiller<'a, G: GradientLike> {
    start_pos: Point,
    idx: usize,
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
            start_pos: gradient.transform * Point::new(f64::from(start_x), f64::from(start_y)),
            range_idx: 0,
            cur_range: &gradient.ranges[0],
            idx: 0,
            gradient,
            kind,
        }
    }

    #[inline]
    fn calc_pos(&self) -> Point {
        let col_idx = self.idx >> (Tile::HEIGHT.trailing_zeros() as usize);
        let row_idx = self.idx & (Tile::HEIGHT as usize - 1);

        self.start_pos
            + self.gradient.x_advance * col_idx as f64
            + self.gradient.y_advance * row_idx as f64
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

        if T::IS_FLOAT {
            target
                .chunks_exact_mut(TILE_HEIGHT_COMPONENTS)
                .for_each(|column| {
                    self.run_column_float::<T>(column);
                });
        } else {
            target
                .chunks_exact_mut(TILE_HEIGHT_COMPONENTS)
                .for_each(|column| {
                    self.run_column_integer::<T>(column, &mut storage);
                });
        }
    }

    fn run_column_float<T: Type>(&mut self, col: &mut [T::Scalar]) {
        let pad = self.gradient.pad;

        for pixel in col.chunks_exact_mut(T::LENGTH) {
            let pos = self.calc_pos();
            let res = self.single::<T::Float>(&pos, pad);
            let converted = T::from_float(&[res]);
            converted.store(pixel);

            self.idx += T::Float::LENGTH / COLOR_COMPONENTS;
        }
    }

    fn run_column_integer<T: Type>(&mut self, col: &mut [T::Scalar], storage: &mut [f32]) {
        let pad = self.gradient.pad;

        for part in storage.chunks_exact_mut(T::Float::LENGTH) {
            let pos = self.calc_pos();
            let res = self.single::<T::Float>(&pos, pad);
            res.store(part);

            self.idx += T::Float::LENGTH / COLOR_COMPONENTS;
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
