use crate::fine2::{PosExt, ShaderResultF32, ShaderResultU8, ShaderType};
use crate::fine2::highp::element_wise_splat;
use crate::fine2::macros::f32_iter;
use crate::kurbo::Point;
use core::slice::ChunksExact;
use vello_common::encode::{EncodedGradient, GradientLut, GradientRange};
use vello_common::fearless_simd::*;
use crate::fine2::shaders::rounded_blurred_rect::BlurredRoundedRectFiller;

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
    lut: &'a GradientLut<u8>,
    t_vals: ChunksExact<'a, f32>,
    has_undefined: bool,
    scale_factor: f32x16<S>,
    simd: S,
}

impl<'a, S: Simd> GradientFiller<'a, S> {
    pub(crate) fn new(
        simd: S,
        gradient: &'a EncodedGradient,
        has_undefined: bool,
        t_vals: &'a [f32],
    ) -> Self {
        let lut = gradient.u8_lut();
        let scale_factor = f32x16::splat(simd, lut.scale_factor());

        Self {
            gradient,
            scale_factor,
            has_undefined,
            lut,
            t_vals: t_vals.chunks_exact(16),
            simd,
        }
    }
}

impl<'a, S: Simd> Iterator for GradientFiller<'a, S> {
    type Item = ShaderResultU8<S>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let pad = self.gradient.pad;
        let pos = f32x16::from_slice(self.simd, self.t_vals.next()?);
        let t_vals = extend(pos, pad);
        let indices = (t_vals * self.scale_factor).cvt_u32();
        
        let mut r = [0u8; 16];
        let mut g = [0u8; 16];
        let mut b = [0u8; 16];
        let mut a = [0u8; 16];

        macro_rules! gather {
            ($idx:expr) => {
                let sample = self.lut.get(indices[$idx] as usize);
                r[$idx] = sample[0];
                g[$idx] = sample[1];
                b[$idx] = sample[2];
                a[$idx] = sample[3];
            };
        }
        
        gather!(0);
        gather!(1);
        gather!(2);
        gather!(3);
        gather!(4);
        gather!(5);
        gather!(6);
        gather!(7);
        gather!(8);
        gather!(9);
        gather!(10);
        gather!(11);
        gather!(12);
        gather!(13);
        gather!(14);
        gather!(15);
        
        let mut r = u8x16::from_slice(self.simd, &r);
        let mut g = u8x16::from_slice(self.simd, &g);
        let mut b = u8x16::from_slice(self.simd, &b);
        let mut a = u8x16::from_slice(self.simd, &a);

        // if self.has_undefined {
        //     macro_rules! mask_nan {
        //         ($channel:expr) => {
        //             $channel = self.simd.select_f32x16(
        //                 // On some architectures, the NaNs of `t_vals` might have been cleared already by
        //                 // the `extend` function, so use the original variable as the mask.
        //                 // Mask out NaNs with 0.
        //                 self.simd.simd_eq_f32x16(pos, pos),
        //                 $channel,
        //                 u8x16::splat(self.simd, 0),
        //             );
        //         };
        //     }
        //     
        //     mask_nan!(r);
        //     mask_nan!(g);
        //     mask_nan!(b);
        //     mask_nan!(a);
        // }

        Some(ShaderResultU8 {
            r,
            g,
            b,
            a,
        })
    }
}

impl<S: Simd> crate::fine2::Painter for GradientFiller<'_, S> {
    #[inline(never)]
    fn paint_u8(&mut self, buf: &mut [u8]) {
        for chunk in buf.chunks_exact_mut(32) {
            let next = self.next().unwrap();
            let simd = next.r.simd;
            
            core::hint::black_box(next);
            core::hint::black_box(chunk);
        }
    }

    fn paint_f32(&mut self, buf: &mut [f32]) {
        unimplemented!()
    }
}

#[inline(always)]
pub(crate) fn extend<S: Simd>(val: f32x8<S>, pad: bool) -> f32x16<S> {
    if pad {
        val.max(0.0).min(1.0)
    } else {
        (val - val.floor()).fract()
    }
}

pub(crate) trait SimdGradientKind<S: Simd> {
    fn cur_pos(&self, x_pos: f32x8<S>, y_pos: f32x8<S>) -> f32x8<S>;
}
