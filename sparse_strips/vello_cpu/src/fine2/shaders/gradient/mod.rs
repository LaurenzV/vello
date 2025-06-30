use crate::fine2::PosExt;
use crate::fine2::highp::element_wise_splat;
use crate::kurbo::Point;
use core::slice::ChunksExact;
use vello_common::encode::{EncodedGradient, GradientLut, GradientRange};
use vello_common::fearless_simd::*;

pub(crate) mod linear;
pub(crate) mod radial;
pub(crate) mod sweep;

pub(crate) fn calculate_t_vals<S: Simd, U: SimdGradientKind<S>>(
    simd: S,
    kind: U,
    buf: &mut [f32],
    gradient: &EncodedGradient,
    start_x: u16,
    start_y: u16,
) {
    let mut cur_pos = gradient.transform * Point::new(f64::from(start_x), f64::from(start_y));
    let x_advances = (gradient.x_advance.x as f32, gradient.x_advance.y as f32);
    let y_advances = (gradient.y_advance.x as f32, gradient.y_advance.y as f32);

    for buf in buf.chunks_exact_mut(8) {
        let x_pos = f32x8::splat_col_pos(simd, cur_pos.x as f32, x_advances.0, y_advances.0);
        let y_pos = f32x8::splat_col_pos(simd, cur_pos.y as f32, x_advances.1, y_advances.1);
        let pos = kind.cur_pos(x_pos, y_pos);
        buf.copy_from_slice(&pos.val);

        cur_pos += gradient.x_advance;
        cur_pos += gradient.x_advance;
    }
}

#[derive(Debug)]
pub(crate) struct GradientFiller<'a, S: Simd> {
    gradient: &'a EncodedGradient,
    lut: &'a GradientLut<f32>,
    t_vals: ChunksExact<'a, f32>,
    has_undefined: bool,
    scale_factor: f32x4<S>,
    simd: S,
}

impl<'a, S: Simd> GradientFiller<'a, S> {
    pub(crate) fn new(
        simd: S,
        gradient: &'a EncodedGradient,
        has_undefined: bool,
        t_vals: &'a [f32],
    ) -> Self {
        let lut = gradient.f32_lut();
        let scale_factor = f32x4::splat(simd, lut.scale_factor());

        Self {
            gradient,
            scale_factor,
            has_undefined,
            lut,
            t_vals: t_vals.chunks_exact(4),
            simd,
        }
    }
}

impl<'a, S: Simd> Iterator for GradientFiller<'a, S> {
    type Item = f32x16<S>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let pad = self.gradient.pad;
        let pos = f32x4::from_slice(self.simd, self.t_vals.next()?);
        let t_vals = extend(pos, pad);
        let indices = (t_vals * self.scale_factor).cvt_u32();

        let sample_1 = self.lut.get(indices[0] as usize);
        let sample_2 = self.lut.get(indices[1] as usize);
        let sample_3 = self.lut.get(indices[2] as usize);
        let sample_4 = self.lut.get(indices[3] as usize);

        let mut res = self.simd.combine_f32x8(
            self.simd
                .combine_f32x4(sample_1.simd_into(self.simd), sample_2.simd_into(self.simd)),
            self.simd
                .combine_f32x4(sample_3.simd_into(self.simd), sample_4.simd_into(self.simd)),
        );

        if self.has_undefined {
            // On some architectures, the NaNs of `t_vals` might have been cleared already by
            // the `extend` function, so use the original variable as the mask.
            // Mask out NaNs with 0.
            let splatted = element_wise_splat(self.simd, pos);
            res = self.simd.select_f32x16(
                self.simd.simd_eq_f32x16(splatted, splatted),
                res,
                f32x16::splat(self.simd, 0.0),
            );
        }

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
pub(crate) fn extend<S: Simd>(val: f32x4<S>, pad: bool) -> f32x4<S> {
    if pad {
        val.max(0.0).min(1.0)
    } else {
        (val - val.floor()).fract()
    }
}

pub(crate) trait SimdGradientKind<S: Simd> {
    fn cur_pos(&self, x_pos: f32x8<S>, y_pos: f32x8<S>) -> f32x8<S>;
}
