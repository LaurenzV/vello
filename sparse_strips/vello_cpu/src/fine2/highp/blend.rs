use crate::peniko::{BlendMode, Mix};
use crate::util::Premultiply;
use vello_common::fearless_simd::*;

#[derive(Copy, Clone)]
struct Channels<S: Simd> {
    r: f32x4<S>,
    g: f32x4<S>,
    b: f32x4<S>,
}

impl<S: Simd> Channels<S> {
    #[inline(always)]
    fn unpremultiply(mut self, a: f32x4<S>) -> Self {
        self.r = self.r.unpremultiply(a);
        self.g = self.g.unpremultiply(a);
        self.b = self.b.unpremultiply(a);
        
        self
    }
}

pub(crate) fn mix<S: Simd>(src_c: f32x16<S>, bg: f32x16<S>, blend_mode: BlendMode) -> f32x16<S> {
    if blend_mode.mix == Mix::Normal {
        return src_c;
    }
    // See https://www.w3.org/TR/compositing-1/#blending
    let simd = src_c.simd;
    
    let split = |input: f32x16<S>| {
        let mut storage = [0.0; 16];
        simd.store_interleaved_128_f32x16(input, &mut storage);
        let input = f32x16::from_slice(simd, &storage);
        
        let p1 = simd.split_f32x16(input);
        let (r, g) = simd.split_f32x8(p1.0);
        let (b, a) = simd.split_f32x8(p1.1);

        (Channels {
            r,
            g,
            b,
        }, a)
    };
    
    let (mut bg_channels, bg_a) = split(bg);
    let (src_channels, src_a) = split(src_c);

    // For blending, we need to first unpremultiply everything.
    let mix_bg = bg_channels.unpremultiply(bg_a);
    let unpremultiplied_src_c = src_channels.unpremultiply(src_a);
    let mut mix_src = unpremultiplied_src_c;
    
    let mut res_bg = mix_bg;

    // Mix the source and background color. This will then be our
    // new source color.
    // Note that mixing should not affect the alpha value, but since we currently
    // SIMDify across the pixel range, the alphas will also be affected. Because of that,
    // we will reset the alpha later to
    mix_src = blend_mode.mix(mix_src, mix_bg);
    
    let apply_alpha = |unpre_src_c: f32x4<S>, mut mix_src: f32x4<S>, dest: &mut f32x4<S>, alpha: f32x4<S>| {
        let p1 = (1.0 - bg_a) * unpre_src_c;
        let p2 = bg_a * mix_src;
        mix_src = p1 + p2;

        *dest = mix_src.premultiply(src_a)
    };
    
    apply_alpha(unpremultiplied_src_c.r, mix_src.r, &mut res_bg.r, bg_a);
    apply_alpha(unpremultiplied_src_c.g, mix_src.g, &mut res_bg.g, bg_a);
    apply_alpha(unpremultiplied_src_c.b, mix_src.b, &mut res_bg.b, bg_a);
    
    let combined = simd.combine_f32x8(
        simd.combine_f32x4(res_bg.r, res_bg.g),
        simd.combine_f32x4(res_bg.b, src_a)
    );
    
    let mut storage = [0.0; 16];
    simd.store_interleaved_128_f32x16(combined, &mut storage);
    f32x16::from_slice(simd, &storage)
}

trait MixExt {
    fn mix<S: Simd>(&self, src: Channels<S>, bg: Channels<S>) -> Channels<S>;
}

impl MixExt for BlendMode {
    fn mix<S: Simd>(&self, src: Channels<S>, bg: Channels<S>) -> Channels<S> {
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
            _ => unimplemented!(),
            // Mix::Hue => Hue::mix(src, bg),
            // Mix::Saturation => Saturation::mix(src, bg),
            // Mix::Color => Color::mix(src, bg),
            // Mix::Luminosity => Luminosity::mix(src, bg),
        }
    }
}

impl Multiply {
    #[inline(always)]
    fn single<S: Simd>(src: f32x4<S>, bg: f32x4<S>) -> f32x4<S> {
        src * bg
    }
}

impl Screen {
    #[inline(always)]
    fn single<S: Simd>(src: f32x4<S>, bg: f32x4<S>) -> f32x4<S> {
        bg + src - src * bg
    }
}

impl HardLight {
    fn single<S: Simd>(src: f32x4<S>, bg: f32x4<S>) -> f32x4<S> {
        let two = f32x4::splat(src.simd, 2.0);

        let mask = src.simd.simd_le_f32x4(src, f32x4::splat(src.simd, 0.5));
        let opt1 = Multiply::single(bg, src * two);
        let opt2 = Screen::single(bg, two * src - 1.0);

        src.simd.select_f32x4(mask, opt1, opt2)
    }
}

macro_rules! separable_mix {
    ($name:ident, $calc:expr) => {
        pub(crate) struct $name;

        impl $name {
            #[inline(always)]
            fn mix<S: Simd>(mut src: Channels<S>, bg: Channels<S>) -> Channels<S> {
                src.r = $calc(src.r, bg.r);
                src.g = $calc(src.g, bg.g);
                src.b = $calc(src.b, bg.b);
                
                src
            }
        }
    };
}

separable_mix!(Multiply, |cs: f32x4<S>, cb: f32x4<S>| Multiply::single(
    cs, cb
));
separable_mix!(Screen, |cs: f32x4<S>, cb: f32x4<S>| Screen::single(
    cs, cb
));
separable_mix!(Overlay, |cs: f32x4<S>, cb: f32x4<S>| HardLight::single(
    cb, cs
));
separable_mix!(Darken, |cs: f32x4<S>, cb: f32x4<S>| cs.min(cb));
separable_mix!(Lighten, |cs: f32x4<S>, cb: f32x4<S>| cs.max(cb));
separable_mix!(Difference, |cs: f32x4<S>, cb: f32x4<S>| {
    cs.simd
        .select_f32x4(cs.simd.simd_le_f32x4(cs, cb), cb - cs, cs - cb)
});
separable_mix!(HardLight, |cs: f32x4<S>, cb: f32x4<S>| HardLight::single(
    cs, cb
));
separable_mix!(Exclusion, |cs: f32x4<S>, cb: f32x4<S>| {
    (cs + cb) - 2.0 * (cs * cb)
});
separable_mix!(SoftLight, |cs: f32x4<S>, cb: f32x4<S>| {
    let mask_1 = cs.simd.simd_le_f32x4(cb, f32x4::splat(cs.simd, 0.25));

    let d = cs
        .simd
        .select_f32x4(mask_1, ((16.0 * cb - 12.0) * cb + 4.0) * cb, cb.sqrt());

    let mask_2 = cs.simd.simd_le_f32x4(cs, f32x4::splat(cs.simd, 0.5));
    let res = cs.simd.select_f32x4(
        mask_2,
        cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb),
        cb + (2.0 * cs - 1.0) * (d - cb),
    );

    res
});
separable_mix!(ColorDodge, |cs: f32x4<S>, cb: f32x4<S>| {
    let mask_1 = cb.simd.simd_eq_f32x4(cb, f32x4::splat(cb.simd, 0.0));
    let mask_2 = cs.simd.simd_eq_f32x4(cs, f32x4::splat(cs.simd, 1.0));

    cs.simd.select_f32x4(
        // if cb == 0
        mask_1,
        f32x4::splat(cs.simd, 0.0),
        // else if cs == 1
        cs.simd.select_f32x4(
            mask_2,
            f32x4::splat(cs.simd, 1.0),
            // else
            f32x4::splat(cs.simd, 1.0).min(cb / (1.0 - cs)),
        ),
    )
});
separable_mix!(ColorBurn, |cs: f32x4<S>, cb: f32x4<S>| {
    let mask_1 = cb.simd.simd_eq_f32x4(cb, f32x4::splat(cb.simd, 1.0));
    let mask_2 = cs.simd.simd_eq_f32x4(cs, f32x4::splat(cs.simd, 0.0));

    cs.simd.select_f32x4(
        // if cb == 1
        mask_1,
        f32x4::splat(cs.simd, 1.0),
        // else if cs == 0
        cs.simd.select_f32x4(
            mask_2,
            f32x4::splat(cs.simd, 0.0),
            // else
            1.0 - f32x4::splat(cs.simd, 1.0).min((1.0 - cb) / cs),
        ),
    )
});

macro_rules! non_separable_mix {
    ($name:ident, $calc:expr) => {
        pub(crate) struct $name;

        impl $name {
            #[inline(always)]
            fn mix<S: Simd>(mut src: f32x4<S>, bg: f32x4<S>, r: f32x4<S>, g: f32x4<S>, b: f32x4<S>) -> f32x16<S> {
                for (src, bg) in (src.val.chunks_exact_mut(4)).zip(bg.val.chunks_exact(4)) {
                    let src_val = src.try_into().unwrap();
                    src.copy_from_slice(&$calc(src_val, bg.try_into().unwrap()));
                }

                src
            }
        }
    };
}
// 
// non_separable_mix!(Hue, |cs, cb| set_lum(set_sat(cs, sat(cb)), lum(cb)));
// non_separable_mix!(Saturation, |cs, cb| set_lum(set_sat(cb, sat(cs)), lum(cb)));
// non_separable_mix!(Color, |cs, cb| set_lum(cs, lum(cb)));
// non_separable_mix!(Luminosity, |cs, cb| set_lum(cb, lum(cs)));
// 
// fn lum<S: Simd>(r: f32x4<S>, g: f32x4<S>, b: f32x4<S>) -> f32x4<S> {
//     0.3 * r + 0.59 * g + 0.11 * b
// }
// 
// fn sat<S: Simd>(r: f32x4<S>, g: f32x4<S>, b: f32x4<S>) -> f32x4<S> {
//     r.max(g).max(b) - r.min(g).min(b)
// }
// 
// fn clip_color<S: Simd>(src: f32x4<S>, r: f32x4<S>, g: f32x4<S>, b: f32x4<S>) -> f32x4<S> {
//     let simd = src.simd;
//     let mut c_new = src;
// 
//     let l = lum(r, g, b);
//     let n = r.min(g.min(b));
//     let x = r.max(g.max(b));
// 
//     c_new = simd.select_f32x4(
//         simd.simd_le_f32x4(n, f32x4::splat(simd, 0.0)),
//         l + (((c_new - l) * l) / (l - n)),
//         c_new
//     );
// 
//     simd.select_f32x4(
//         simd.simd_gt_f32x4(x, f32x4::splat(simd, 1.0)),
//         l + (((c_new - l) * (1.0 - l)) / (x - l)),
//         c_new
//     )
// }
// 
// fn set_lum<S: Simd>(mut src: f32x4<S>, r: f32x4<S>, g: f32x4<S>, b: f32x4<S>, l: f32x4<S>) -> f32x4<S> {
//     let d = l - lum(r, g, b);
//     src = src + d;
// 
//     clip_color(src, r, g, b)
// }
// 
// fn set_sat<S: Simd>(mut c: [f32; 4], s: f32) -> [f32; 4] {
//     let (min, tail) = c.split_at_mut(1);
//     let (mid, max) = tail.split_at_mut(1);
// 
//     let mut min = &mut min[0];
//     let mut mid = &mut mid[0];
//     let mut max = &mut max[0];
// 
//     if *min > *mid {
//         core::mem::swap(&mut min, &mut mid);
//     }
// 
//     if *min > *max {
//         core::mem::swap(&mut min, &mut max);
//     }
// 
//     if *mid > *max {
//         core::mem::swap(&mut mid, &mut max);
//     }
// 
//     if *max > *min {
//         *mid = ((*mid - *min) * s) / (*max - *min);
//         *max = s;
//     } else {
//         *mid = 0.0;
//         *max = 0.0;
//     }
// 
//     *min = 0.0;
// 
//     c
// }
