use crate::Paint;
use crate::fine::ScratchBuf;
use std::arch::aarch64::{
    float32x4x2_t, float32x4x4_t, uint8x16x4_t, uint32x4x4_t, vdup_n_u32, vdupq_n_u32, vld1q_f32,
    vreinterpretq_u8_u32, vst1q_f32, vst1q_f32_x2, vst1q_f32_x4, vst1q_u8_x4, vst1q_u32_x4,
};
use vello_simd::Float;

pub const HEIGHT: usize = 4;
pub const WIDETILE_WIDTH: usize = 256;
const COLOR_COMPONENTS: usize = 4;
const TILE_HEIGHT_COMPONENTS: usize = HEIGHT * COLOR_COMPONENTS;
#[doc(hidden)]
pub const SCRATCH_BUF_SIZE: usize = WIDETILE_WIDTH * HEIGHT * COLOR_COMPONENTS;

pub fn opaque_u8(blend_buf: &mut [u8], color: &[u8; 4]) {
    unsafe {
        let loaded = vreinterpretq_u8_u32(vdupq_n_u32(u32::from_be_bytes(*color)));
        let matrix = uint8x16x4_t(loaded, loaded, loaded, loaded);
        let blend_buf = &mut blend_buf[0..][..SCRATCH_BUF_SIZE];

        for t in blend_buf.chunks_exact_mut(64) {
            vst1q_u8_x4(t.as_mut_ptr(), matrix);
        }
    }
}

pub fn opaque_f32(blend_buf: &mut [f32], color: &[f32; 4]) {
    let splat = vello_simd::neon::f32x8::load(color);
    
    for t in blend_buf.array_chunks_mut::<4>() {
        splat.store(t);
    }
}
