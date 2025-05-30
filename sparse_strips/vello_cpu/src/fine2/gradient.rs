use crate::fine2::{COLOR_COMPONENTS, Painter, TILE_HEIGHT_COMPONENTS};
use std::f32::consts::PI;
use std::marker::PhantomData;
use vello_common::encode::{EncodedGradient, GradientLike, GradientRange, LinearKind, SweepKind};
use vello_common::kurbo::Point;
use vello_simd::{ColorLike, Float, NumberKind, Type};

#[derive(Debug)]
pub struct SimdLinearKind<T: Float> {
    kind: LinearKind,
    phantom_data: PhantomData<T>,
}

#[derive(Debug)]
pub struct SimdSweepKind<T: Float> {
    start_angle: T,
    inv_angle_delta: T,
    kind: SweepKind,
}

impl<T: Float> From<LinearKind> for SimdLinearKind<T> {
    fn from(value: LinearKind) -> Self {
        Self {
            phantom_data: PhantomData::default(),
            kind: value,
        }
    }
}

impl<T: Float> From<SweepKind> for SimdSweepKind<T> {
    fn from(value: SweepKind) -> Self {
        Self {
            start_angle: T::splat(value.start_angle),
            inv_angle_delta: T::splat(value.inv_angle_delta),
            kind: value,
        }
    }
}

trait SimdGradientKind<T: Float> {
    fn cur_pos(&self, x_pos: T, y_pos: T) -> T;
    fn cur_pos_scalar(&self, point: Point) -> f32;
}

impl<T: Float> SimdGradientKind<T> for SimdLinearKind<T> {
    fn cur_pos(&self, x_pos: T, _: T) -> T {
        x_pos
    }

    fn cur_pos_scalar(&self, point: Point) -> f32 {
        self.kind.cur_pos(point)
    }
}

impl<T: Float> SimdGradientKind<T> for SimdSweepKind<T> {
    fn cur_pos(&self, x_pos: T, y_pos: T) -> T {
        let angle = x_y_to_unit_angle(x_pos, y_pos * T::splat(-1.0)) * T::splat(2.0 * PI);

        (angle - self.start_angle) * self.inv_angle_delta
    }

    fn cur_pos_scalar(&self, point: Point) -> f32 {
        self.kind.cur_pos(point)
    }
}

#[derive(Debug)]
pub(crate) struct GradientFiller<'a, S: Type, U: SimdGradientKind<S::Float>> {
    cur_pos: Point,
    idx: usize,
    gradient: &'a EncodedGradient,
    kind: U,
    x_advances: (f32, f32),
    y_advances: (f32, f32),
    cur_ranges: Vec<(usize, &'a GradientRange)>,
    stored_t_vals: &'a mut [f32; 16],
    c0: &'a mut [f32; 64],
    x0: &'a mut [f32; 64],
    factors: &'a mut [f32; 64],
    phantom_data: PhantomData<S>,
}

impl<'a, S: Type, U: SimdGradientKind<S::Float>> GradientFiller<'a, S, U> {
    pub(crate) fn new(
        gradient: &'a EncodedGradient,
        kind: &'a (impl Into<U> + Copy),
        temp_buf: &'a mut [f32],
        start_x: u16,
        start_y: u16,
    ) -> Self {
        let (stored_t_vals, tail) = temp_buf.split_first_chunk_mut::<16>().unwrap();
        let (c0, tail) = tail.split_first_chunk_mut::<64>().unwrap();
        let (x0, tail) = tail.split_first_chunk_mut::<64>().unwrap();
        let (factors, _) = tail.split_first_chunk_mut::<64>().unwrap();

        Self {
            cur_pos: gradient.transform * Point::new(f64::from(start_x), f64::from(start_y)),
            cur_ranges: vec![],
            stored_t_vals,
            c0,
            x0,
            factors,
            idx: 0,
            gradient,
            x_advances: (gradient.x_advance.x as f32, gradient.x_advance.y as f32),
            y_advances: (gradient.y_advance.x as f32, gradient.y_advance.y as f32),
            kind: (*kind).into(),
            phantom_data: PhantomData::default(),
        }
    }

    pub(super) fn run(mut self, target: &mut [S::Scalar]) {
        let pad = self.gradient.pad;

        let bl = self.gradient.y_advance * 4.0;
        let tr = self.gradient.x_advance * 4.0;
        let br = bl + tr;

        let mut cur_range = InnerRange::new(0, &self.gradient.ranges);
        let mut bottom_range = InnerRange::new(0, &self.gradient.ranges);

        target.chunks_exact_mut(64).for_each(|column| {
            let cur_pos = self.kind.cur_pos_scalar(self.cur_pos);
            let cur_pos_extended = extend_f32(cur_pos, pad);
            advance(cur_pos_extended, &mut cur_range, &self.gradient.ranges);

            let bot_pos = self.kind.cur_pos_scalar(self.cur_pos + bl);
            let bot_pos_extended = extend_f32(bot_pos, pad);
            advance(bot_pos_extended, &mut bottom_range, &self.gradient.ranges);

            fn check_advance(
                cur_pos: f32,
                cur_pos_extended: f32,
                end_pos: f32,
                cur_range: &InnerRange,
            ) -> bool {
                let delta = end_pos - cur_pos;
                let to_check = cur_pos_extended + delta;

                to_check < cur_range.x0 || to_check >= cur_range.x1
            }

            let tlbr_advance = check_advance(
                cur_pos,
                cur_pos_extended,
                self.kind.cur_pos_scalar(self.cur_pos + br),
                &cur_range,
            );
            let bltr_advance = check_advance(
                bot_pos,
                bot_pos_extended,
                self.kind.cur_pos_scalar(self.cur_pos + tr),
                &bottom_range,
            );

            if S::IS_FLOAT {
                if tlbr_advance || bltr_advance {
                    self.run_float_range_scalar(column, cur_range);
                } else {
                    self.run_float_range(column, &cur_range);
                }
            } else {
                self.run_float_range_scalar(column, cur_range);
            }

            self.cur_pos += self.gradient.x_advance * 4.0;
        });
    }

    fn run_float_range(&mut self, target: &mut [S::Scalar], range: &InnerRange) {
        let pad = self.gradient.pad;
        let (x_pos, y_pos) = S::Float::splat_col_pos(
            (self.cur_pos.x as f32, self.cur_pos.y as f32),
            self.x_advances,
            self.y_advances,
        );

        let x0 = range.x0::<S::Float>();
        let c0 = range.c0::<S::Float>();
        let factors = range.factors::<S::Float>();

        let t_vals = extend(self.kind.cur_pos(x_pos, y_pos), pad);
        t_vals.store(self.stored_t_vals);

        for (t, target) in self
            .stored_t_vals
            .chunks_exact(4)
            .zip(target.chunks_exact_mut(S::LENGTH))
        {
            let t_vals = S::Float::load_alphas_f32(t);

            let added = factors.mul_add(t_vals - x0, c0);
            let converted = S::from_float(&[added]);
            converted.store(target);
        }
    }

    fn run_float_range_scalar(&mut self, target: &mut [S::Scalar], mut inner: InnerRange) {
        let pad = self.gradient.pad;
        let mut cur_pos = self.cur_pos;

        for col in target.chunks_exact_mut(TILE_HEIGHT_COMPONENTS) {
            let mut temp_pos = cur_pos;

            for pixel in col.chunks_exact_mut(COLOR_COMPONENTS) {
                let t_val = extend_f32(self.kind.cur_pos_scalar(temp_pos), pad);
                advance(t_val, &mut inner, &self.gradient.ranges);

                let x0 = inner.x0;
                let c0 = inner.c0;
                let factors = inner.factors;

                for (idx, c) in pixel.iter_mut().enumerate() {
                    let factor = factors[idx] * (t_val - x0);
                    let added = c0[idx] + factor;

                    *c = S::Scalar::from_normalized_f32(added)
                }

                temp_pos += self.gradient.y_advance;
            }

            cur_pos += self.gradient.x_advance;
        }
    }
}

#[derive(Copy, Clone)]
struct InnerRange {
    idx: usize,
    x0: f32,
    x1: f32,
    c0: [f32; 4],
    factors: [f32; 4],
}

impl InnerRange {
    #[inline]
    pub fn new(idx: usize, ranges: &[GradientRange]) -> Self {
        let range = &ranges[idx];

        Self {
            idx,
            x0: range.x0,
            x1: range.x1,
            c0: range.c0.to_rgbf32(),
            factors: range.factors_f32,
        }
    }

    pub fn x0<S: Float>(&self) -> S {
        S::splat(self.x0)
    }

    pub fn c0<S: Float>(&self) -> S {
        S::splat_4(self.c0)
    }

    pub fn factors<S: Float>(&self) -> S {
        S::splat_4(self.factors)
    }
}

#[inline]
fn advance(target_pos: f32, inner: &mut InnerRange, ranges: &[GradientRange]) {
    let mut range_idx = inner.idx;
    let mut cur_range = &ranges[range_idx];

    while target_pos < cur_range.x0 {
        range_idx -= 1;
        cur_range = &ranges[range_idx];
    }

    while target_pos >= cur_range.x1 {
        range_idx += 1;
        cur_range = &ranges[range_idx];
    }

    if range_idx != inner.idx {
        *inner = InnerRange::new(range_idx, ranges);
    }
}

impl<F: Type, U: SimdGradientKind<F::Float>> Painter<F> for GradientFiller<'_, F, U> {
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

#[inline]
pub(crate) fn extend_f32(mut val: f32, pad: bool) -> f32 {
    if pad {
        val
    } else {
        (val - val.floor()).fract()
    }
}

fn x_y_to_unit_angle<T: Float>(x: T, y: T) -> T {
    let c0 = T::splat(0.0);
    let c1 = T::splat(1.0);
    let c2 = T::splat(1.0 / 4.0);
    let c3 = T::splat(1.0 / 2.0);

    let x_abs = x.abs();
    let y_abs = y.abs();

    let slope = x_abs.min(y_abs) / x_abs.max(y_abs);
    let s = slope * slope;

    let a = s.mul_add(
        T::splat(-7.0547382347285747528076171875e-3),
        T::splat(2.476101927459239959716796875e-2),
    );
    let b = s.mul_add(a, T::splat(-5.185396969318389892578125e-2));
    let c = s.mul_add(b, T::splat(0.15912117063999176025390625));

    let mut phi = slope * c;

    phi = T::if_then_else(x_abs.lt(y_abs) , c2 - phi, phi);
    phi = T::if_then_else(x.lt(c0), c3 - phi, phi);
    phi = T::if_then_else(y.lt(c0), c1 - phi, phi);
    phi = T::if_then_else(phi.ne(phi), c0, phi);

    phi
}
