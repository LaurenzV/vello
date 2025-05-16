#![allow(missing_docs)]

use std::arch::aarch64::{uint8x16x4_t, vdupq_n_u32, vreinterpretq_u8_u32, vst1q_u8_x4};

pub const HEIGHT: usize = 4;
pub const WIDETILE_WIDTH: usize = 256;
const COLOR_COMPONENTS: usize = 4;
const TILE_HEIGHT_COMPONENTS: usize = HEIGHT * COLOR_COMPONENTS;
#[doc(hidden)]
pub const SCRATCH_BUF_SIZE: usize = WIDETILE_WIDTH * HEIGHT * COLOR_COMPONENTS;

#[inline(never)]
pub fn opaque_u8(blend_buf: &mut [u8], color: u32, color_u8: [u8; 4], width: usize) {
    unsafe {
        // let single = vreinterpretq_u8_u32(vdupq_n_u32(color));
        let single = vreinterpretq_u8_u32(vdupq_n_u32(u32::from_be_bytes(color_u8)));
        let m = uint8x16x4_t(single, single, single, single);

        for c in blend_buf[..width * HEIGHT * COLOR_COMPONENTS].chunks_exact_mut(64) {
            vst1q_u8_x4(c.as_mut_ptr(), m);
        }
    }

    // 5.3 ns
    // for c in blend_buf[..width * HEIGHT * COLOR_COMPONENTS].chunks_exact_mut(4) {
    //     c.copy_from_slice(color);
    // }
}

#[inline(never)]
pub fn opaque_f32(blend_buf: &mut [f32], color: &[f32; 4]) {
    for c in blend_buf.chunks_exact_mut(4) {
        c.copy_from_slice(color);
    }
}
