use crate::fine2::{COLOR_COMPONENTS, Painter, TILE_HEIGHT_COMPONENTS};
use std::f32::consts::PI;
use std::marker::PhantomData;
use vello_common::encode::{
    EncodedGradient, FocalData, GradientLike, GradientRange, LinearKind, RadialKind, SweepKind,
};
use vello_common::kurbo::Point;
use vello_simd::{ColorLike, Float, Mask, NumberKind, Type};

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

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct SimdFocalData<T: Float> {
    focal_data: FocalData,
    fr1: T,
    f_focal_x: T,
    f_is_swapped: T::Mask,
}

pub enum SimdRadialKindInner<T: Float> {
    Radial {
        bias: T,
        scale: T,
    },
    Strip {
        scaled_r0_squared: T,
    },
    Focal {
        focal_data: SimdFocalData<T>,
        fp0: T,
        fp1: T,
    },
}

pub struct SimdRadialKind<T: Float> {
    inner: SimdRadialKindInner<T>,
    kind: RadialKind
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

impl<T: Float> From<RadialKind> for SimdRadialKind<T> {
    fn from(value: RadialKind) -> Self {
        let inner = match value {
            RadialKind::Radial { bias, scale } => SimdRadialKindInner::Radial {
                bias: T::splat(bias),
                scale: T::splat(scale),
            },
            RadialKind::Strip { scaled_r0_squared } => SimdRadialKindInner::Strip {
                scaled_r0_squared: T::splat(scaled_r0_squared),
            },
            RadialKind::Focal {
                focal_data,
                fp0,
                fp1,
            } => SimdRadialKindInner::Focal {
                fp0: T::splat(fp0),
                fp1: T::splat(fp1),
                focal_data: SimdFocalData {
                    focal_data,
                    fr1: T::splat(focal_data.fr1),
                    f_focal_x: T::splat(focal_data.f_focal_x),
                    f_is_swapped: T::Mask::splat(focal_data.f_is_swapped),
                },
            },
        };
        
        SimdRadialKind {
            inner,
            kind: value,
        }
    }
}

trait SimdGradientKind<T: Float> {
    fn cur_pos(&self, x_pos: T, y_pos: T) -> T;
    fn cur_pos_mask(&self, _: T, _: T) -> T {
        T::splat(1.0)
    }
    fn cur_pos_scalar(&self, point: Point) -> f32;
    fn cur_pos_mask_scalar(&self, point: Point) -> f32;
    fn has_undefined(&self) -> bool {
        false
    }
}

impl<T: Float> SimdGradientKind<T> for SimdLinearKind<T> {
    fn cur_pos(&self, x_pos: T, _: T) -> T {
        x_pos
    }

    fn cur_pos_scalar(&self, point: Point) -> f32 {
        self.kind.cur_pos(point)
    }

    fn cur_pos_mask_scalar(&self, point: Point) -> f32 {
        self.kind.cur_pos_mask(point)
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

    fn cur_pos_mask_scalar(&self, point: Point) -> f32 {
        self.kind.cur_pos_mask(point)
    }
}

impl<T: Float> SimdGradientKind<T> for SimdRadialKind<T> {
    fn cur_pos(&self, x_pos: T, y_pos: T) -> T {
        match &self.inner {
            SimdRadialKindInner::Radial { bias, scale } => {
                let mut radius = (x_pos * x_pos + y_pos * y_pos).sqrt();

                *bias + radius * *scale
            }
            SimdRadialKindInner::Strip { scaled_r0_squared } => {
                let p1 = *scaled_r0_squared - y_pos * y_pos;

                x_pos + p1.sqrt()
            }
            SimdRadialKindInner::Focal {
                focal_data,
                fp0,
                fp1,
            } => {
                let mut t = if focal_data.focal_data.is_focal_on_circle() {
                    x_pos + y_pos * y_pos / x_pos
                } else if focal_data.focal_data.is_well_behaved() {
                    (x_pos * x_pos + y_pos * y_pos).sqrt() - x_pos * *fp0
                } else if focal_data.focal_data.is_swapped() || (1.0 - focal_data.focal_data.f_focal_x < 0.0) {
                    T::splat(-1.0) * (x_pos * x_pos - y_pos * y_pos).sqrt() - x_pos * *fp0
                } else {
                    (x_pos * x_pos - y_pos * y_pos).sqrt() - x_pos * *fp0
                };

                if 1.0 - focal_data.focal_data.f_focal_x < 0.0 {
                    t = T::splat(-1.0) * t;
                }

                if !focal_data.focal_data.is_natively_focal() {
                    t = t + *fp1;
                }

                if focal_data.focal_data.is_swapped() {
                    t = T::splat(1.0) - t;
                }

                t
            }
        }
    }

    fn cur_pos_scalar(&self, point: Point) -> f32 {
        self.kind.cur_pos(point)
    }

    fn cur_pos_mask(&self, x_pos: T, y_pos: T) -> T {
        todo!()
    }

    fn has_undefined(&self) -> bool {
        self.kind.has_undefined()
    }

    fn cur_pos_mask_scalar(&self, point: Point) -> f32 {
        1.0
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
        let original_pos = self.cur_pos;
        
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

                to_check < cur_range.range.x0 || to_check >= cur_range.range.x1
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
                self.run_float_range_scalar(column, cur_range);
            } else {
                self.run_float_range_scalar(column, cur_range);
            }

            self.cur_pos += self.gradient.x_advance * 4.0;
        });
        
        // if self.kind.has_undefined() {
        //     let mut cur_pos = original_pos;
        // 
        //     for col in target.chunks_exact_mut(TILE_HEIGHT_COMPONENTS) {
        //         let mut temp_pos = cur_pos;
        // 
        //         for pixel in col.chunks_exact_mut(COLOR_COMPONENTS) {
        //             let mask = S::Scalar::from_normalized_f32(self.kind.cur_pos_scalar(temp_pos));
        //             
        //             for c in pixel {
        //                 *c = c.normalized_mul(mask);
        //             }
        // 
        //             temp_pos += self.gradient.y_advance;
        //         }
        // 
        //         cur_pos += self.gradient.x_advance;
        //     }
        // }
    }

    fn run_float_range(&mut self, target: &mut [S::Scalar], range: &InnerRange) {
        let pad = self.gradient.pad;
        let (x_pos, y_pos) = S::Float::splat_col_pos(
            (self.cur_pos.x as f32, self.cur_pos.y as f32),
            self.x_advances,
            self.y_advances,
        );

        let bias = range.bias::<S::Float>();
        let scale = range.scale::<S::Float>();

        let t_vals = extend(self.kind.cur_pos(x_pos, y_pos), pad);
        t_vals.store(self.stored_t_vals);

        for (t, target) in self
            .stored_t_vals
            .chunks_exact(4)
            .zip(target.chunks_exact_mut(S::LENGTH))
        {
            let t_vals = S::Float::load_alphas_f32(t);

            let res = t_vals.mul_add(scale, bias);
            let converted = S::from_float(&[res]);
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

                let bias = inner.range.bias;
                let scale = inner.range.scale;

                for (comp_idx, comp) in pixel.iter_mut().enumerate() {
                    *comp = S::Scalar::from_normalized_f32(bias[comp_idx] + scale[comp_idx] * t_val);
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
    range: GradientRange
}

impl InnerRange {
    #[inline]
    pub fn new(idx: usize, ranges: &[GradientRange]) -> Self {
        let range = ranges[idx].clone();

        Self {
            idx,
            range
        }
    }

    pub fn bias<S: Float>(&self) -> S {
        S::splat_4(self.range.bias)
    }

    pub fn scale<S: Float>(&self) -> S {
        S::splat_4(self.range.scale)
    }
}

#[inline]
fn advance<'a>(target_pos: f32, inner: &mut InnerRange, ranges: &[GradientRange]) {
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

    phi = T::if_then_else(x_abs.lt(y_abs), c2 - phi, phi);
    phi = T::if_then_else(x.lt(c0), c3 - phi, phi);
    phi = T::if_then_else(y.lt(c0), c1 - phi, phi);
    phi = T::if_then_else(phi.ne(phi), c0, phi);

    phi
}
