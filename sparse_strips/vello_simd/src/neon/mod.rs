mod f32x16;
mod f32x4;
mod f32x8;
mod u8x16;
mod u8x32;
mod u8x64;

use crate::Simd;
use std::arch::aarch64::{
    float32x4_t, uint16x8_t, vaddq_u16, vdupq_n_f32, vdupq_n_u16, vfmaq_f32, vld1q_f32, vshrq_n_u16,
};
use std::arch::is_aarch64_feature_detected;

#[derive(Copy, Clone, Debug)]
pub struct Neon(NeonImpl);

impl Neon {
    pub fn new() -> Option<Self> {
        if is_aarch64_feature_detected!("neon") {
            Some(Self(NeonImpl))
        } else {
            None
        }
    }

    pub fn get(&self) -> impl Simd + Sized + use<> {
        self.0
    }
}

#[derive(Copy, Clone, Debug)]
struct NeonImpl;

impl Simd for NeonImpl {
    type Integer = u8x64::u8x64;
    type Float = f32x16::f32x16;
}

#[inline(always)]
fn div_255(input: uint16x8_t) -> uint16x8_t {
    unsafe {
        let p1 = vdupq_n_u16(255);
        let p2 = vaddq_u16(input, p1);
        vshrq_n_u16::<8>(p2)
    }
}

#[inline(always)]
fn splat_col_pos(pos: (f32, f32), advance: (f32, f32)) -> (float32x4_t, float32x4_t) {
    unsafe {
        let column_mask = vld1q_f32([0.0, 1.0, 2.0, 3.0].as_ptr());

        let x_positions = vfmaq_f32(vdupq_n_f32(pos.0), column_mask, vdupq_n_f32(advance.0));
        let y_positions = vfmaq_f32(vdupq_n_f32(pos.1), column_mask, vdupq_n_f32(advance.1));

        (x_positions, y_positions)
    }
}
