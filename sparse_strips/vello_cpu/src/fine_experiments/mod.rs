use crate::Paint;
use crate::fine::ScratchBuf;
use std::arch::aarch64::{
    float32x4x2_t, float32x4x4_t, uint8x16x4_t, uint32x4x4_t, vdup_n_u32, vdupq_n_u32, vld1q_f32,
    vreinterpretq_u8_u32, vst1q_f32, vst1q_f32_x2, vst1q_f32_x4, vst1q_u8_x4, vst1q_u32_x4,
};
use vello_simd::Narrowed;

pub const HEIGHT: usize = 4;
pub const WIDETILE_WIDTH: usize = 256;
const COLOR_COMPONENTS: usize = 4;
const TILE_HEIGHT_COMPONENTS: usize = HEIGHT * COLOR_COMPONENTS;
#[doc(hidden)]
pub const SCRATCH_BUF_SIZE: usize = WIDETILE_WIDTH * HEIGHT * COLOR_COMPONENTS;

pub fn opaque_u8(blend_buf: &mut [u8], color: &[u8; 4]) {
    let splat = vello_simd::neon::u8x64::load_4(color);

    for t in blend_buf.array_chunks_mut::<64>() {
        splat.store(t);
    }
}

pub fn opaque_f32(blend_buf: &mut [f32], color: &[f32; 4]) {
    let splat = vello_simd::neon::f32x8::load_4(color);

    for t in blend_buf.array_chunks_mut::<8>() {
        splat.store(t);
    }
}
