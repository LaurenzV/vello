use crate::peniko::{BlendMode, Mix};
use vello_common::fearless_simd::*;

pub(crate) trait MixExt {
    fn mix<S: Simd>(&self, src: f32x16<S>, bg: f32x16<S>) -> f32x16<S>;
}

impl MixExt for BlendMode {
    fn mix<S: Simd>(&self, src: f32x16<S>, bg: f32x16<S>) -> f32x16<S> {
        match self.mix {
            Mix::Normal => src,
            // Same as `Normal`.
            Mix::Clip => src,
            Mix::Multiply => Multiply::mix(src, bg),
            Mix::Screen => Screen::mix(src, bg),
            Mix::Overlay => Overlay::mix(src, bg),
            Mix::Darken => Darken::mix(src, bg),
            Mix::Lighten => Lighten::mix(src, bg),
            Mix::ColorDodge => ColorDodge::mix(src, bg),
            Mix::ColorBurn => ColorBurn::mix(src, bg),
            Mix::HardLight => HardLight::mix(src, bg),
            Mix::SoftLight => SoftLight::mix(src, bg),
            Mix::Difference => Difference::mix(src, bg),
            Mix::Exclusion => Exclusion::mix(src, bg),
            _ => src,
            // Mix::Hue => Hue::mix(src, bg),
            // Mix::Saturation => Saturation::mix(src, bg),
            // Mix::Color => Color::mix(src, bg),
            // Mix::Luminosity => Luminosity::mix(src, bg),
        }
    }
}

impl Multiply {
    #[inline(always)]
    fn single<S: Simd>(src: f32x16<S>, bg: f32x16<S>) -> f32x16<S> {
        src * bg
    }
}

impl Screen {
    #[inline(always)]
    fn single<S: Simd>(src: f32x16<S>, bg: f32x16<S>) -> f32x16<S> {
        bg + src - src * bg
    }
}

impl HardLight {
    fn single<S: Simd>(src: f32x16<S>, bg: f32x16<S>) -> f32x16<S> {
        let two = f32x16::splat(src.simd, 2.0);

        let mask = src.simd.simd_le_f32x16(src, f32x16::splat(src.simd, 0.5));
        let opt1 = Multiply::single(bg, src * two);
        let opt2 = Screen::single(bg, two * src - 1.0);

        src.simd.select_f32x16(mask, opt1, opt2)
    }
}

macro_rules! separable_mix {
    ($name:ident, $calc:expr) => {
        pub(crate) struct $name;

        impl $name {
            #[inline(always)]
            fn mix<S: Simd>(src: f32x16<S>, bg: f32x16<S>) -> f32x16<S> {
                $calc(src, bg)
            }
        }
    };
}

separable_mix!(Multiply, |cs: f32x16<S>, cb: f32x16<S>| Multiply::single(
    cs, cb
));
separable_mix!(Screen, |cs: f32x16<S>, cb: f32x16<S>| Screen::single(
    cs, cb
));
separable_mix!(Overlay, |cs: f32x16<S>, cb: f32x16<S>| HardLight::single(
    cb, cs
));
separable_mix!(Darken, |cs: f32x16<S>, cb: f32x16<S>| cs.min(cb));
separable_mix!(Lighten, |cs: f32x16<S>, cb: f32x16<S>| cs.max(cb));
separable_mix!(Difference, |cs: f32x16<S>, cb: f32x16<S>| {
    cs.simd
        .select_f32x16(cs.simd.simd_le_f32x16(cs, cb), cb - cs, cs - cb)
});
separable_mix!(HardLight, |cs: f32x16<S>, cb: f32x16<S>| HardLight::single(
    cs, cb
));
separable_mix!(Exclusion, |cs: f32x16<S>, cb: f32x16<S>| {
    (cs + cb) - 2.0 * (cs * cb)
});
separable_mix!(SoftLight, |cs: f32x16<S>, cb: f32x16<S>| {
    let mask_1 = cs.simd.simd_le_f32x16(cb, f32x16::splat(cs.simd, 0.25));

    let d = cs
        .simd
        .select_f32x16(mask_1, ((16.0 * cb - 12.0) * cb + 4.0) * cb, cb.sqrt());

    let mask_2 = cs.simd.simd_le_f32x16(cs, f32x16::splat(cs.simd, 0.5));
    let res = cs.simd.select_f32x16(
        mask_2,
        cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb),
        cb + (2.0 * cs - 1.0) * (d - cb),
    );

    res
});
separable_mix!(ColorDodge, |cs: f32x16<S>, cb: f32x16<S>| {
    let mask_1 = cb.simd.simd_eq_f32x16(cb, f32x16::splat(cb.simd, 0.0));
    let mask_2 = cs.simd.simd_eq_f32x16(cs, f32x16::splat(cs.simd, 1.0));
    
    cs.simd.select_f32x16(
        // if cb == 0
        mask_1, 
        f32x16::splat(cs.simd, 0.0),
        // else if cs == 1
        cs.simd.select_f32x16(
            mask_2,
            f32x16::splat(cs.simd, 1.0),
            // else
            f32x16::splat(cs.simd, 1.0)
            .min(cb / (1.0 - cs))
        )
    )
});
separable_mix!(ColorBurn, |cs: f32x16<S>, cb: f32x16<S>| {
    let mask_1 = cb.simd.simd_eq_f32x16(cb, f32x16::splat(cb.simd, 1.0));
    let mask_2 = cs.simd.simd_eq_f32x16(cs, f32x16::splat(cs.simd, 0.0));
    
    cs.simd.select_f32x16(
        // if cb == 1
        mask_1, 
        f32x16::splat(cs.simd, 1.0),
        // else if cs == 0
        cs.simd.select_f32x16(
            mask_2,
            f32x16::splat(cs.simd, 0.0),
            // else
            (1.0 - f32x16::splat(cs.simd, 1.0).min((1.0 - cb) / cs))
        )
    )
});
