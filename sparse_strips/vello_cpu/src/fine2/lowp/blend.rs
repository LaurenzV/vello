use vello_common::fearless_simd::*;
use crate::fine2::Splat4thExt;
use crate::peniko::{BlendMode, Compose};
use crate::util::NormalizedMulExt;

// 
// // pub(crate) mod fill {
// //     use crate::fine2::blend::BlendModeExt;
// //     use vello_common::peniko::BlendMode;
// //     use vello_simd::Type;
// // 
// //     pub(crate) fn blend<N: Type, T: Iterator<Item = N>>(
// //         target: &mut [N::Scalar],
// //         src_c: T,
// //         blend_mode: BlendMode,
// //     ) {
// //         for (part, src_c) in target.chunks_exact_mut(N::LENGTH).zip(src_c) {
// //             let mut bg_c = N::load(part);
// //             blend_mode.compose(src_c, &mut bg_c, N::one());
// //             bg_c.store(part);
// //         }
// //     }
// // }
// // 
// // pub(crate) mod strip {
// //     use crate::fine2::blend::BlendModeExt;
// //     use vello_common::peniko::BlendMode;
// //     use vello_simd::Type;
// // 
// //     pub(crate) fn blend<N: Type, T: Iterator<Item = N>>(
// //         target: &mut [N::Scalar],
// //         src_c: T,
// //         alphas: &[u8],
// //         blend_mode: BlendMode,
// //     ) {
// //         for ((bg_part, masks), src_c) in target
// //             .chunks_exact_mut(N::LENGTH)
// //             .zip(alphas.chunks_exact(N::LENGTH / 4))
// //             .zip(src_c)
// //         {
// //             let mut bg_c = N::load(bg_part);
// //             let mask_a = N::load_alphas(masks);
// // 
// //             blend_mode.compose(src_c, &mut bg_c, mask_a);
// //             bg_c.store(bg_part);
// //         }
// //     }
// // 
// //     // pub(crate) fn blend<
// //     //     F: FineType,
// //     //     T: Iterator<Item = [F; COLOR_COMPONENTS]>,
// //     //     A: Iterator<Item = [u8; Tile::HEIGHT as usize]>,
// //     // >(
// //     //     target: &mut [F],
// //     //     mut color_iter: T,
// //     //     blend_mode: BlendMode,
// //     //     mut alphas: A,
// //     // ) {
// //     //     for bg_col in target.chunks_exact_mut(TILE_HEIGHT_COMPONENTS) {
// //     //         let masks = alphas.next().unwrap();
// //     //
// //     //         for (bg_pix, mask) in bg_col.chunks_exact_mut(Tile::HEIGHT as usize).zip(masks) {
// //     //             blend_mode.compose(&mixed_src_color, bg_pix, F::from_normalized_u8(mask));
// //     //         }
// //     //     }
// //     // }
// // }
// 
// pub(crate) trait BlendModeExt {
//     fn compose<S: Simd>(&self, simd: S, src_c: u8x32<S>, bg_c: u8x32<S>, alpha_mask: u8x32<S>) -> u8x32<S>;
// }
// 
// impl BlendModeExt for BlendMode {
//     fn compose<S: Simd>(&self, simd: S, src_c: u8x32<S>, bg_c: u8x32<S>, alpha_mask: u8x32<S>) -> u8x32<S> {
//         match self.compose {
//             Compose::SrcOver => SrcOver::compose(simd, src_c, bg_c, alpha_mask),
//             Compose::Clear => Clear::compose(simd, src_c, bg_c, alpha_mask),
//             Compose::Copy => Copy::compose(simd, src_c, bg_c, alpha_mask),
//             Compose::DestOver => DestOver::compose(simd, src_c, bg_c, alpha_mask),
//             Compose::Dest => Dest::compose(simd, src_c, bg_c, alpha_mask),
//             Compose::SrcIn => SrcIn::compose(simd, src_c, bg_c, alpha_mask),
//             Compose::DestIn => DestIn::compose(simd, src_c, bg_c, alpha_mask),
//             Compose::SrcOut => SrcOut::compose(simd, src_c, bg_c, alpha_mask),
//             Compose::DestOut => DestOut::compose(simd, src_c, bg_c, alpha_mask),
//             Compose::SrcAtop => SrcAtop::compose(simd, src_c, bg_c, alpha_mask),
//             Compose::DestAtop => DestAtop::compose(simd, src_c, bg_c, alpha_mask),
//             Compose::Xor => Xor::compose(simd, src_c, bg_c, alpha_mask),
//             Compose::Plus => Plus::compose(simd, src_c, bg_c, alpha_mask),
//             // Have not been able to find a formula for this, so just fallback to Plus.
//             Compose::PlusLighter => SrcOver::compose(src_c, bg_c, alpha_mask),
//         }
//     }
// }
// 
macro_rules! compose {
    ($name:ident, $fa:expr, $fb:expr, $sat:expr) => {
        struct $name;

        impl $name {
            fn compose<S: Simd>(simd: S, src_c: u8x32<S>, bg_c: u8x32<S>, mask: u8x32<S>) -> u8x32<S> {
                let al_b = bg_c.splat_4th();
                let al_s = src_c.splat_4th().normalized_mul(mask);
        
                let fa = $fa(simd, al_s, al_b);
                let fb = $fb(simd, al_s, al_b);
        
                let src_c = src_c.normalized_mul(mask);
        
                if $sat {
                    simd.narrow_u16x32(
                        simd.widen_u8x32(src_c.normalized_mul(fa)) + simd.widen_u8x32(fb.normalized_mul(bg_c))
                            .min(u16x32::splat(simd, 255))
                            .max(u16x32::splat(simd, 0))
                    )
                } else {
                    src_c.normalized_mul(fa) + fb.normalized_mul(bg_c)
                }
            }
        }
    };
}


compose!(Clear, |simd, _, _| u8x32::splat(simd, 0), |simd, _, _| u8x32::splat(simd, 0), false);
compose!(Copy, |simd, _, _| u8x32::splat(simd, 255), |simd, _, _| u8x32::splat(simd, 0), false);
compose!(
    SrcOver,
    |simd, _, _| u8x32::splat(simd, 255),
    |simd, al_s: u8x32<S>, _| 255 - al_s,
    false
);
compose!(
    DestOver,
    |simd, _, al_b: u8x32<S>| 255 - al_b,
    |simd, _, _| u8x32::splat(simd, 255),
    false
);
compose!(Dest, |simd, _, _| u8x32::splat(simd, 0), |simd, _, _| u8x32::splat(simd, 255), false);
compose!(
    Xor,
    |simd, _, al_b: u8x32<S>| 255 - al_b,
    |simd, al_s: u8x32<S>, _| 255 - al_s,
    false
);
compose!(SrcIn, |simd, _, al_b: u8x32<S>| al_b, |simd, _, _| u8x32::splat(simd, 0), false);
compose!(DestIn, |simd, _, _| u8x32::splat(simd, 0), |simd, al_s: u8x32<S>, _| al_s, false);
compose!(
    SrcOut,
    |simd, _, al_b: u8x32<S>| 255 - al_b,
    |simd, _, _| u8x32::splat(simd, 0),
    false
);
compose!(
    DestOut,
    |simd, _, _| u8x32::splat(simd, 0),
    |simd, al_s: u8x32<S>, _| 255 - al_s,
    false
);
compose!(
    SrcAtop,
    |simd, _, al_b: u8x32<S>| al_b,
    |simd, al_s: u8x32<S>, _| 255 - al_s,
    false
);
compose!(
    DestAtop,
    |simd, _, al_b: u8x32<S>| 255 - al_b,
    |simd, al_s: u8x32<S>, _| al_s,
    false
);
compose!(Plus, |simd, _, _| 255, |simd, _, _| 255, true);