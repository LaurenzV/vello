// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::{FINE_ITERS, SEED};
use criterion::Criterion;
use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};
use vello_common::coarse::WideTile;
use vello_common::color::palette::css::{BLUE, GREEN, RED, ROYAL_BLUE, YELLOW};
use vello_common::paint::{LinearGradient, Paint, Stop};
use vello_common::peniko;
use vello_common::tile::Tile;
use vello_cpu::fine::Fine;

pub fn fill(c: &mut Criterion) {
    let mut g = c.benchmark_group("fine/fill");

    macro_rules! fill_single {
        ($name:ident, $paint:expr) => {
            g.bench_function(stringify!($name), |b| {
                b.iter(|| {
                    let mut out = vec![];
                    let mut fine = Fine::new(WideTile::WIDTH, Tile::HEIGHT, &mut out);

                    for _ in 0..FINE_ITERS {
                        fine.fill(0, 0, WideTile::WIDTH as usize, $paint);
                    }
                })
            });
        };
    }

    fill_single!(opaque, &ROYAL_BLUE.into());
    fill_single!(transparent, &ROYAL_BLUE.with_alpha(0.2).into());

    let linear: Paint = LinearGradient {
        x0: 0.0,
        x1: WideTile::WIDTH as f32,
        stops: stops_blue_green_red_yellow(),
        extend: peniko::Extend::Pad
    }
    .into();

    fill_single!(linear_gradient, &linear);
}

pub fn strip(c: &mut Criterion) {
    let mut g = c.benchmark_group("fine/strip");
    let mut rng = StdRng::from_seed(SEED);

    let mut alphas = vec![];

    for _ in 0..WideTile::WIDTH * Tile::HEIGHT {
        alphas.push(rng.random());
    }

    macro_rules! strip_single {
        ($name:ident, $paint:expr) => {
            g.bench_function(stringify!($name), |b| {
                b.iter(|| {
                    let mut out = vec![];
                    let mut fine = Fine::new(WideTile::WIDTH, Tile::HEIGHT, &mut out);

                    for _ in 0..FINE_ITERS {
                        fine.strip(0, 0, WideTile::WIDTH as usize, &alphas, $paint);
                    }
                })
            });
        };
    }

    strip_single!(basic, &ROYAL_BLUE.into());

    let linear: Paint = LinearGradient {
        x0: 0.0,
        x1: WideTile::WIDTH as f32,
        stops: stops_blue_green_red_yellow(),
        extend: peniko::Extend::Pad,
    }
    .into();

    strip_single!(linear_gradient, &linear);
}

fn stops_blue_green_red_yellow() -> Vec<Stop> {
    vec![
        Stop {
            offset: 0.0,
            color: BLUE,
        },
        Stop {
            offset: 0.33,
            color: GREEN,
        },
        Stop {
            offset: 0.66,
            color: RED,
        },
        Stop {
            offset: 1.0,
            color: YELLOW,
        },
    ]
}
