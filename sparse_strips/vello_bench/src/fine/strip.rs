use crate::SEED;
use crate::fine::default_blend;
use criterion::{Bencher, Criterion};
use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};
use vello_common::coarse::WideTile;
use vello_common::color::palette::css::ROYAL_BLUE;
use vello_common::encode::EncodedPaint;
use vello_common::paint::{Paint, PremulColor};
use vello_common::tile::Tile;
use vello_cpu::fine2::{Fine, SimdExt};
use vello_dev_macros::vello_bench;
use vello_simd::Narrowed;

pub fn strip(c: &mut Criterion) {
    solid_short(c);
    solid_long(c);
}

#[vello_bench]
pub fn solid_short<const C: usize, const A: usize, N: Narrowed<C, A> + SimdExt>(b: &mut Bencher<'_>, fine: &mut Fine<C, A, N>) {
    let paint = Paint::Solid(PremulColor::from_alpha_color(ROYAL_BLUE));
    let width = 8;

    strip_single(&paint, &[], width, b, fine);
}

#[vello_bench]
pub fn solid_long<const C: usize, const A: usize, N: Narrowed<C, A> + SimdExt>(b: &mut Bencher<'_>, fine: &mut Fine<C, A, N>) {
    let paint = Paint::Solid(PremulColor::from_alpha_color(ROYAL_BLUE));
    let width = 64;

    strip_single(&paint, &[], width, b, fine);
}

fn strip_single<const C: usize, const A: usize, N: Narrowed<C, A> + SimdExt>(
    paint: &Paint,
    encoded_paints: &[EncodedPaint],
    width: usize,
    b: &mut Bencher<'_>,
    fine: &mut Fine<C, A, N>,
) {
    let mut rng = StdRng::from_seed(SEED);
    let mut alphas = vec![];

    for _ in 0..WideTile::WIDTH * Tile::HEIGHT {
        alphas.push(rng.random());
    }

    b.iter(|| {
        fine.strip(0, width, &alphas, paint, default_blend(), encoded_paints);

        std::hint::black_box(&fine);
    })
}
