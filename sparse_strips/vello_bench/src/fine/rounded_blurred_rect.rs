use crate::fine::{default_blend, fill_single};
use criterion::{Bencher, Criterion};
use vello_common::blurred_rounded_rect::BlurredRoundedRectangle;
use vello_common::coarse::WideTile;
use vello_common::color::palette::css::GREEN;
use vello_common::encode::EncodeExt;
use vello_common::kurbo::{Affine, Point, Rect};
use vello_common::tile::Tile;
use vello_cpu::fine2::Fine;
use vello_dev_macros::vello_bench;
use vello_simd::Type;

pub fn rounded_blurred_rect(c: &mut Criterion) {
    with_transform(c);
    no_transform(c);
}

#[vello_bench]
fn with_transform<N: Type>(b: &mut Bencher<'_>, fine: &mut Fine<N>) {
    let center = Point::new(WideTile::WIDTH as f64 / 2.0, Tile::HEIGHT as f64 / 2.0);

    base(b, fine, Affine::rotate_about(1.0, center));
}

#[vello_bench]
fn no_transform<N: Type>(b: &mut Bencher<'_>, fine: &mut Fine<N>) {
    base(b, fine, Affine::IDENTITY)
}

fn base<F: Type>(b: &mut Bencher<'_>, fine: &mut Fine<F>, transform: Affine) {
    let mut paints = vec![];

    let rect = BlurredRoundedRectangle {
        rect: Rect::new(0.0, 0.0, WideTile::WIDTH as f64, Tile::HEIGHT as f64),
        color: GREEN,
        radius: 30.0,
        std_dev: 10.0,
    };

    let paint = rect.encode_into(&mut paints, transform);
    fill_single(
        &paint,
        &paints,
        WideTile::WIDTH as usize,
        b,
        default_blend(),
        fine,
    );
}
