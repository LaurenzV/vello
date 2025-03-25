// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::{FINE_ITERS, SEED};
use criterion::Criterion;
use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};
use vello_common::coarse::WideTile;
use vello_common::color::palette::css::{BLUE, GREEN, RED, ROYAL_BLUE, YELLOW};
use vello_common::kurbo::Point;
use vello_common::paint::{LinearGradient, Paint, Stop};
use vello_common::peniko;
use vello_common::tile::Tile;
use vello_cpu::fine::Fine;
use vello_cpu::paint::EncodedPaint;

pub fn fill(c: &mut Criterion) {
    let mut g = c.benchmark_group("fine/fill");

    macro_rules! fill_single {
        ($name:ident, $paint:expr) => {
            g.bench_function(stringify!($name), |b| {
                b.iter(|| {
                    let mut out = vec![];
                    let mut fine = Fine::new(WideTile::WIDTH, Tile::HEIGHT, &mut out);

                    for _ in 0..FINE_ITERS {
                        fine.fill(0, 0, 0, WideTile::WIDTH as usize, $paint);
                    }
                })
            });
        };
    }

    fill_single!(opaque, &ROYAL_BLUE.into());
    fill_single!(transparent, &ROYAL_BLUE.with_alpha(0.2).into());

    macro_rules! fill_single_linear {
        ($name:ident, $extend:ident, $stops:expr) => {
            let linear: EncodedPaint = LinearGradient {
                p0: Point::new(80.0, 0.0),
                p1: Point::new(120.0, 0.0),
                stops: $stops,
                extend: peniko::Extend::$extend,
            }
            .into();

            fill_single!($name, &linear);
        };
    }

    fill_single_linear!(
        linear_gradient_opaque,
        Pad,
        stops_blue_green_red_yellow_opaque()
    );
    fill_single_linear!(linear_gradient_pad, Pad, stops_blue_green_red_yellow());
    fill_single_linear!(
        linear_gradient_repeat,
        Repeat,
        stops_blue_green_red_yellow()
    );
    // Reflect is just a special case of repeat, so no extra benchmarks.
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
                        fine.strip(0, 0, 0, WideTile::WIDTH as usize, &alphas, $paint);
                    }
                })
            });
        };
    }

    strip_single!(basic, &ROYAL_BLUE.into());

    macro_rules! strip_single_linear {
        ($name:ident, $extend:ident) => {
            let linear: EncodedPaint = LinearGradient {
                p0: Point::new(0.0, 0.0),
                p1: Point::new(WideTile::WIDTH as f64, 0.0),
                stops: stops_blue_green_red_yellow(),
                extend: peniko::Extend::$extend,
            }
            .into();

            strip_single!($name, &linear);
        };
    }

    strip_single_linear!(linear_gradient_pad, Pad);
    strip_single_linear!(linear_gradient_repeat, Repeat);
    // Reflect is just a special case of repeat, so not extra benchmarks.
}

fn stops_blue_green_red_yellow_opaque() -> Vec<Stop> {
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

fn stops_blue_green_red_yellow() -> Vec<Stop> {
    vec![
        Stop {
            offset: 0.0,
            color: BLUE,
        },
        Stop {
            offset: 0.33,
            color: GREEN.with_alpha(0.5),
        },
        Stop {
            offset: 0.66,
            color: RED,
        },
        Stop {
            offset: 1.0,
            color: YELLOW.with_alpha(0.7),
        },
    ]
}
