use criterion::{Bencher, Criterion};
use vello_common::color::palette::css::{GREEN, TRANSPARENT};
use vello_common::paint::PremulColor;
use vello_dev_macros::vello_bench;
use vello_simd::Type;

pub fn clear(c: &mut Criterion) {
    clear_normal(c);
    clear_transparent(c);
}

#[vello_bench]
pub fn clear_normal<N: Type>(b: &mut Bencher<'_>, fine: &mut vello_cpu::fine2::Fine<N>) {
    let color = PremulColor::from_alpha_color(GREEN);

    b.iter(|| {
        fine.clear(&color);
    });
}

#[vello_bench]
pub fn clear_transparent<N: Type>(b: &mut Bencher<'_>, fine: &mut vello_cpu::fine2::Fine<N>) {
    let color = PremulColor::from_alpha_color(TRANSPARENT);

    b.iter(|| {
        fine.clear(&color);
    });
}