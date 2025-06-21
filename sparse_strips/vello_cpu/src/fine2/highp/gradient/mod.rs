use vello_common::encode::{EncodedGradient, GradientRange};
use vello_common::fearless_simd::*;
use crate::fine2::highp::{calc_pos, element_wise_splat};
use crate::fine2::PosExt;
use crate::kurbo::Point;

pub(crate) mod linear;

#[derive(Debug)]
pub(crate) struct GradientFiller<'a, S: Simd, U: SimdGradientKind<S>> {
    start_pos: Point,
    idx: usize,
    gradient: &'a EncodedGradient,
    kind: U,
    x_advances: (f32, f32),
    y_advances: (f32, f32),
    simd: S
}

impl<'a, S: Simd, U: SimdGradientKind<S>> GradientFiller<'a, S, U> {
    pub(crate) fn new(simd: S,
        gradient: &'a EncodedGradient,
        kind: &'a (impl Into<U> + Copy),
        start_x: u16,
        start_y: u16,
    ) -> Self {
        let start_pos = gradient.transform * Point::new(f64::from(start_x), f64::from(start_y));
        
        Self {
            start_pos,
            idx: 0,
            gradient,
            x_advances: (gradient.x_advance.x as f32, gradient.x_advance.y as f32),
            y_advances: (gradient.y_advance.x as f32, gradient.y_advance.y as f32),
            kind: (*kind).into(),
            simd
        }
    }
}

impl<'a, S: Simd, U: SimdGradientKind<S>> Iterator for GradientFiller<'a, S, U> {
    type Item = f32x16<S>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let pad = self.gradient.pad;
        let cur_pos = calc_pos(self.start_pos, self.idx, self.gradient.x_advance, self.gradient.y_advance);
        let x_pos = f32x4::splat_col_pos(self.simd, cur_pos.x as f32, self.x_advances.0, self.y_advances.0);
        let y_pos = f32x4::splat_col_pos(self.simd, cur_pos.y as f32, self.x_advances.1, self.y_advances.1);
        let t_vals = extend(self.kind.cur_pos(x_pos, y_pos), pad);
        let indices = advance(self.simd, t_vals, &self.gradient.ranges);

        let r0 = &self.gradient.ranges[indices[0] as usize];
        let r1 = &self.gradient.ranges[indices[1] as usize];
        let r2 = &self.gradient.ranges[indices[2] as usize];
        let r3 = &self.gradient.ranges[indices[3] as usize];
        
        let t_vals = element_wise_splat(self.simd, t_vals);

        let scales = self.simd.combine_f32x8(
            self.simd.combine_f32x4(r0.scale.simd_into(self.simd), r1.scale.simd_into(self.simd)),
            self.simd.combine_f32x4(r2.scale.simd_into(self.simd), r3.scale.simd_into(self.simd)),
        );

        let biases = self.simd.combine_f32x8(
            self.simd.combine_f32x4(r0.bias.simd_into(self.simd), r1.bias.simd_into(self.simd)),
            self.simd.combine_f32x4(r2.bias.simd_into(self.simd), r3.bias.simd_into(self.simd)),
        );
        
        let res = biases.madd(scales, t_vals);
        self.idx += 4;
        
        Some(res)
    }
}

#[inline(always)]
fn advance<S: Simd>(simd: S, target_pos: f32x4<S>, ranges: &[GradientRange]) -> u32x4<S> {
    let mut idx = u32x4::splat(simd, 0);

    for i in 0..(ranges.len() - 1) {
        let cond = simd.simd_le_f32x4(f32x4::splat(simd, ranges[i].x1), target_pos);
        idx = idx + simd.select_u32x4(cond, u32x4::splat(simd, 1), u32x4::splat(simd, 0));
    }

    idx
}


#[inline(always)]
pub(crate) fn extend<S: Simd>(mut val: f32x4<S>, pad: bool) -> f32x4<S> {
    if pad {
        val
    } else {
        (val - val.floor()).fract()
    }
}

trait SimdGradientKind<S: Simd> {
    fn cur_pos(&self, x_pos: f32x4<S>, y_pos: f32x4<S>) -> f32x4<S>;
    fn has_undefined(&self) -> bool {
        false
    }
}
