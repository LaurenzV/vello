use vello_common::peniko::{BlendMode, Compose};
use vello_simd::{Type, Widened};

pub(crate) mod fill {
    use crate::fine2::blend::BlendModeExt;
    use vello_common::peniko::BlendMode;
    use vello_simd::Type;

    pub(crate) fn blend<N: Type, T: Iterator<Item = N>>(
        target: &mut [N::Scalar],
        src_c: T,
        blend_mode: BlendMode,
    ) {
        for (part, src_c) in target.chunks_exact_mut(N::LENGTH).zip(src_c) {
            let mut bg_c = N::load(part);
            blend_mode.compose(src_c, &mut bg_c, N::one());
            bg_c.store(part);
        }
    }
}

pub(crate) mod strip {
    use crate::fine2::blend::BlendModeExt;
    use vello_common::peniko::BlendMode;
    use vello_simd::Type;

    pub(crate) fn blend<N: Type, T: Iterator<Item = N>>(
        target: &mut [N::Scalar],
        src_c: T,
        alphas: &[u8],
        blend_mode: BlendMode,
    ) {
        for ((bg_part, masks), src_c) in target
            .chunks_exact_mut(N::LENGTH)
            .zip(alphas.chunks_exact(N::LENGTH / 4))
            .zip(src_c)
        {
            let mut bg_c = N::load(bg_part);
            let mask_a = N::load_alphas(masks);

            blend_mode.compose(src_c, &mut bg_c, mask_a);
            bg_c.store(bg_part);
        }
    }

    // pub(crate) fn blend<
    //     F: FineType,
    //     T: Iterator<Item = [F; COLOR_COMPONENTS]>,
    //     A: Iterator<Item = [u8; Tile::HEIGHT as usize]>,
    // >(
    //     target: &mut [F],
    //     mut color_iter: T,
    //     blend_mode: BlendMode,
    //     mut alphas: A,
    // ) {
    //     for bg_col in target.chunks_exact_mut(TILE_HEIGHT_COMPONENTS) {
    //         let masks = alphas.next().unwrap();
    //
    //         for (bg_pix, mask) in bg_col.chunks_exact_mut(Tile::HEIGHT as usize).zip(masks) {
    //             blend_mode.compose(&mixed_src_color, bg_pix, F::from_normalized_u8(mask));
    //         }
    //     }
    // }
}

pub(crate) trait BlendModeExt {
    fn compose<F: Type>(&self, src_c: F, bg_c: &mut F, alpha_mask: F);
}

impl BlendModeExt for BlendMode {
    fn compose<F: Type>(&self, src_c: F, bg_c: &mut F, alpha_mask: F) {
        match self.compose {
            Compose::SrcOver => SrcOver::compose(src_c, bg_c, alpha_mask),
            Compose::Clear => Clear::compose(src_c, bg_c, alpha_mask),
            Compose::Copy => Copy::compose(src_c, bg_c, alpha_mask),
            Compose::DestOver => DestOver::compose(src_c, bg_c, alpha_mask),
            Compose::Dest => Dest::compose(src_c, bg_c, alpha_mask),
            Compose::SrcIn => SrcIn::compose(src_c, bg_c, alpha_mask),
            Compose::DestIn => DestIn::compose(src_c, bg_c, alpha_mask),
            Compose::SrcOut => SrcOut::compose(src_c, bg_c, alpha_mask),
            Compose::DestOut => DestOut::compose(src_c, bg_c, alpha_mask),
            Compose::SrcAtop => SrcAtop::compose(src_c, bg_c, alpha_mask),
            Compose::DestAtop => DestAtop::compose(src_c, bg_c, alpha_mask),
            Compose::Xor => Xor::compose(src_c, bg_c, alpha_mask),
            Compose::Plus => Plus::compose(src_c, bg_c, alpha_mask),
            // Have not been able to find a formula for this, so just fallback to Plus.
            Compose::PlusLighter => SrcOver::compose(src_c, bg_c, alpha_mask),
        }
    }
}

macro_rules! compose {
    ($name:ident, $fa:expr, $fb:expr, $sat:expr) => {
        struct $name;

        impl $name {
            fn compose<F: Type>(src_c: F, bg_c: &mut F, mask: F) {
                let al_b = bg_c.splat_4th_element();
                let al_s = src_c.splat_4th_element().normalized_mul(mask);

                for i in 0..4 {
                    let fa = $fa(al_s, al_b);
                    let fb = $fb(al_s, al_b);

                    let src_c = src_c.normalized_mul(mask);

                    if $sat {
                        *bg_c = (src_c.normalized_mul(fa).widen()
                            + fb.normalized_mul(*bg_c).widen())
                        .clamp()
                        .narrow();
                    } else {
                        *bg_c = src_c.normalized_mul(fa).add(fb.normalized_mul(*bg_c));
                    }
                }
            }
        }
    };
}

compose!(Clear, |_, _| F::zero(), |_, _| F::zero(), false);
compose!(Copy, |_, _| F::one(), |_, _| F::zero(), false);
compose!(
    SrcOver,
    |_, _| F::one(),
    |al_s: F, _| al_s.one_minus(),
    false
);
compose!(
    DestOver,
    |_, al_b: F| al_b.one_minus(),
    |_, _| F::one(),
    false
);
compose!(Dest, |_, _| F::zero(), |_, _| F::one(), false);
compose!(
    Xor,
    |_, al_b: F| al_b.one_minus(),
    |al_s: F, _| al_s.one_minus(),
    false
);
compose!(SrcIn, |_, al_b: F| al_b, |_, _| F::zero(), false);
compose!(DestIn, |_, _| F::zero(), |al_s: F, _| al_s, false);
compose!(
    SrcOut,
    |_, al_b: F| al_b.one_minus(),
    |_, _| F::zero(),
    false
);
compose!(
    DestOut,
    |_, _| F::zero(),
    |al_s: F, _| al_s.one_minus(),
    false
);
compose!(
    SrcAtop,
    |_, al_b: F| al_b,
    |al_s: F, _| al_s.one_minus(),
    false
);
compose!(
    DestAtop,
    |_, al_b: F| al_b.one_minus(),
    |al_s: F, _| al_s,
    false
);
compose!(Plus, |_, _| F::one(), |_, _| F::one(), true);
