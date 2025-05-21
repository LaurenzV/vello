use std::iter::{once, zip};
use crate::fine2::{COLOR_COMPONENTS, Painter, TILE_HEIGHT_COMPONENTS};
use vello_common::encode::{EncodedGradient, GradientLike, GradientRange, LinearKind};
use vello_common::kurbo::Point;
use vello_simd::{Float, Type};

#[derive(Debug)]
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
pub(crate) struct GradientFiller<'a, S: Type> {
    cur_pos: Point,
    idx: usize,
    gradient: &'a EncodedGradient,
    kind: SimdLinearKind<S::Float>,
    x_advances: (f32, f32),
    y_advances: (f32, f32),
    cur_ranges: Vec<(usize, &'a GradientRange)>,
    stored_t_vals: Vec<f32>,
    c0: Vec<f32>,
    x0: Vec<f32>,
    factors: Vec<f32>
}

impl<'a, S: Type> GradientFiller<'a, S> {
    pub(crate) fn new(
        gradient: &'a EncodedGradient,
        kind: &'a LinearKind,
        start_x: u16,
        start_y: u16,
    ) -> Self {
        Self {
            cur_pos: gradient.transform * Point::new(f64::from(start_x), f64::from(start_y)),
            cur_ranges: vec![],
            stored_t_vals: vec![0.0; 16],
            c0: vec![0.0; 64],
            x0: vec![0.0; 64],
            factors: vec![0.0; 64],
            idx: 0,
            gradient,
            x_advances: (gradient.x_advance.x as f32, gradient.x_advance.y as f32),
            y_advances: (gradient.y_advance.x as f32, gradient.y_advance.y as f32),
            kind: (*kind).into(),
        }
    }

    

    pub(super) fn run(mut self, target: &mut [S::Scalar]) {
        self.cur_ranges = vec![(0, &self.gradient.ranges[0]); 16];
        
        if S::IS_FLOAT {
            target
                .chunks_exact_mut(64)
                .for_each(|column| {
                    self.run_float(column);
                    
                    self.cur_pos += self.gradient.x_advance * 4.0;
                });
        } else {
            target
                .chunks_exact_mut(64)
                .for_each(|column| {
                    self.run_integer(column);

                    self.cur_pos += self.gradient.x_advance * 4.0;
                });
        }
    }

    fn run_float(&mut self, target: &mut [S::Scalar]) {
        let pad = self.gradient.pad;
        let (x_pos, y_pos) = S::Float::splat_col_pos(
            (self.cur_pos.x as f32, self.cur_pos.y as f32),
            self.x_advances,
            self.y_advances,
        );
        
        let t_vals = extend(self.kind.cur_pos(x_pos, y_pos), pad);
        t_vals.store(&mut self.stored_t_vals);
        
        for ((((target_pos, (idx, range)), c0), x0), factors) in self.stored_t_vals.iter()
            .zip(self.cur_ranges.iter_mut())
            .zip(self.c0.chunks_exact_mut(4))
            .zip(self.x0.chunks_exact_mut(4))
            .zip(self.factors.chunks_exact_mut(4))
        {
            advance(*target_pos, idx, range, c0, x0, factors, &self.gradient.ranges);
        }
        
        for ((((t, c0), x0), factors), target) in self.stored_t_vals.chunks_exact(4)
            .zip(self.c0.chunks_exact(S::LENGTH))
            .zip(self.x0.chunks_exact(S::LENGTH))
            .zip(self.factors.chunks_exact(S::LENGTH))
            .zip(target.chunks_exact_mut(S::LENGTH)) {
            let x0 = S::Float::load(x0);
            let c0 = S::Float::load(c0);
            let factors = S::Float::load(factors);
            let t = S::Float::load_alphas_f32(t);

            let factor = factors * (t - x0);
            let added = c0 + factor;
            let converted = S::from_float(&[added]);
            converted.store(target);
        }
    }

    fn run_integer(&mut self, target: &mut [S::Scalar]) {
        let pad = self.gradient.pad;
        let (x_pos, y_pos) = S::Float::splat_col_pos(
            (self.cur_pos.x as f32, self.cur_pos.y as f32),
            self.x_advances,
            self.y_advances,
        );

        let t_vals = extend(self.kind.cur_pos(x_pos, y_pos), pad);
        t_vals.store(&mut self.stored_t_vals);

        for ((((target_pos, (idx, range)), c0), x0), factors) in self.stored_t_vals.iter()
            .zip(self.cur_ranges.iter_mut())
            .zip(self.c0.chunks_exact_mut(4))
            .zip(self.x0.chunks_exact_mut(4))
            .zip(self.factors.chunks_exact_mut(4))
        {
            advance(*target_pos, idx, range, c0, x0, factors, &self.gradient.ranges);
        }
        
        let mut converted = vec![];

        for ((((t, c0), x0), factors)) in self.stored_t_vals.chunks_exact(4)
            .zip(self.c0.chunks_exact(S::Float::LENGTH))
            .zip(self.x0.chunks_exact(S::Float::LENGTH))
            .zip(self.factors.chunks_exact(S::Float::LENGTH)) {
            let x0 = S::Float::load(x0);
            let c0 = S::Float::load(c0);
            let factors = S::Float::load(factors);
            let t = S::Float::load_alphas_f32(t);

            let factor = factors * (t - x0);
            let added = c0 + factor;
            converted.push(added);
        }
        
        let res = S::from_float(&converted);
        res.store(target);
    }
}

#[inline]
fn advance<'a>(target_pos: f32, range_idx: &mut usize, cur_range: &mut &'a GradientRange, c0: &mut [f32], x0: &mut [f32], factors: &mut [f32], ranges: &'a [GradientRange]) {
    while target_pos > cur_range.x1 || target_pos < cur_range.x0 {
        if *range_idx == 0 {
            *range_idx = ranges.len() - 1;
        } else {
            *range_idx -= 1;
        }

        *cur_range = &ranges[*range_idx];
    }

    c0.copy_from_slice(&cur_range.c0.as_premul_f32().components);
    factors.copy_from_slice(&cur_range.factors_f32);
    let x0_n = cur_range.x0;
    x0.copy_from_slice(&[x0_n; 4]);
}

impl<F: Type> Painter<F> for GradientFiller<'_, F> {
    fn paint(self, target: &mut [F::Scalar]) {
        self.run(target);
    }
}

#[inline]
pub(crate) fn extend<T: Float>(mut val: T, pad: bool) -> T {
    if pad {
        val
    } else {
        (val - val.floor()).fract()
    }
}
