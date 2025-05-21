use crate::fine2::{COLOR_COMPONENTS, Painter, TILE_HEIGHT_COMPONENTS};
use vello_common::encode::{EncodedGradient, GradientLike, GradientRange, LinearKind};
use vello_common::kurbo::Point;
use vello_simd::{Float, Type};

pub struct SimdLinearKind<T: Float> {
    inv_distance: T,
    y2_minus_y1: T,
    x2_minus_x1: T,
}

impl<T: Float> From<LinearKind> for SimdLinearKind<T> {
    fn from(value: LinearKind) -> Self {
        Self {
            inv_distance: T::splat(value.inv_distance),
            y2_minus_y1: T::splat(value.y2_minus_y1),
            x2_minus_x1: T::splat(value.x2_minus_x1),
        }
    }
}

trait SimdGradientKind<T: Float> {
    fn cur_pos(&self, x_pos: T, y_pos: T) -> T;
}

impl<T: Float> SimdGradientKind<T> for SimdLinearKind<T> {
    fn cur_pos(&self, x_pos: T, y_pos: T) -> T {
        (x_pos * self.y2_minus_y1 - y_pos * self.x2_minus_x1) * self.inv_distance
    }
}

#[derive(Debug)]
pub(crate) struct GradientFiller<'a, T: Float> {
    cur_pos: Point,
    idx: usize,
    gradient: &'a EncodedGradient,
    kind: SimdLinearKind<T>,
    x_advances: (f32, f32),
    y_advances: (f32, f32),
    cur_ranges: Vec<(usize, &'a GradientRange)>,
}

impl<'a, T: Float> GradientFiller<'a, T> {
    pub(crate) fn new(
        gradient: &'a EncodedGradient,
        kind: &'a LinearKind,
        start_x: u16,
        start_y: u16,
    ) -> Self {
        Self {
            cur_pos: gradient.transform * Point::new(f64::from(start_x), f64::from(start_y)),
            cur_ranges: vec![],
            idx: 0,
            gradient,
            x_advances: (gradient.x_advance.x as f32, gradient.x_advance.y as f32),
            y_advances: (gradient.y_advance.x as f32, gradient.y_advance.y as f32),
            kind: kind.into(),
        }
    }

    fn advance(&self, target_pos: f32, range_idx: &mut usize, cur_range: &mut &'a GradientRange) {
        while target_pos > cur_range.x1 || target_pos < cur_range.x0 {
            if *range_idx == 0 {
                *range_idx = self.gradient.ranges.len() - 1;
            } else {
                *range_idx -= 1;
            }

            *cur_range = &self.gradient.ranges[*range_idx];
        }
    }

    pub(super) fn run<T: Type>(mut self, target: &mut [T::Scalar]) {
        self.cur_ranges = vec![(0, &self.gradient.ranges[0]); T::Float::LENGTH];
        
        if T::IS_FLOAT {
            target
                .chunks_exact_mut(64)
                .for_each(|column| {
                    self.run_float::<T>(column);
                    
                    self.cur_pos += self.gradient.x_advance * 4.0;
                });
        } else {
            unimplemented!()
        }
    }

    fn run_float<T: Type>(&mut self, target: &mut [T::Scalar]) {
        let pad = self.gradient.pad;
        let (x_pos, y_pos) = T::Float::splat_col_pos(
            (self.cur_pos.x as f32, self.cur_pos.y as f32),
            self.x_advances,
            self.y_advances,
        );
        
        let t_vals = self.kind.cur_pos(x_pos, y_pos);
        
        let t = 

        for pixel in col.chunks_exact_mut(T::LENGTH) {
            let res = self.single::<T::Float>(&pos, pad);
            let converted = T::from_float(&[res]);
            converted.store(pixel);

            self.idx += T::Float::LENGTH / COLOR_COMPONENTS;
        }
    }

    // fn run_column_integer<T: Type>(&mut self, col: &mut [T::Scalar], storage: &mut [f32]) {
    //     let pad = self.gradient.pad;
    // 
    //     for part in storage.chunks_exact_mut(T::Float::LENGTH) {
    //         let pos = self.calc_pos();
    //         let res = self.single::<T::Float>(&pos, pad);
    //         res.store(part);
    // 
    //         self.idx += T::Float::LENGTH / COLOR_COMPONENTS;
    //     }
    // 
    //     T::load_f32_many(storage).store(col);
    // }

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

pub(crate) fn extend<T: Float>(mut val: T, pad: bool) -> T {
    if pad {
        val
    } else {
        while val < 0.0 {
            val = 1.0;
        }

        while val > 1.0 {
            val -= 1.0;
        }

        val
    }
}
