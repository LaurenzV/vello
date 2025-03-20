// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![allow(missing_docs, reason = "Not needed for benchmarks")]
#![allow(unreachable_pub, reason = "Otherwise benchmarks won't compile")]

use criterion::Criterion;
use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};
use vello_common::coarse::WideTile;
use vello_common::color::palette::css::{BLUE, GREEN};
use vello_common::color::{AlphaColor, Srgb};
use vello_common::paint::{LinearGradient, Paint, Stop};
use vello_common::tile::Tile;
use vello_cpu::fine::Fine;

const FILL_ITERS: usize = 50;

pub fn fill(c: &mut Criterion) {
    let mut g = c.benchmark_group("fill");

    macro_rules! fill_single {
        ($name:ident, $paint:expr) => {
            g.bench_function(stringify!($name), |b| {
                b.iter(|| {
                    let mut out = vec![];
                    let mut fine = Fine::new(WideTile::WIDTH, Tile::HEIGHT, &mut out);

                    for _ in 0..FILL_ITERS {
                        fine.fill(0, 0, WideTile::WIDTH as usize, $paint);
                    }
                })
            });
        };
    }

    let transparent_solid: Paint = AlphaColor::from_rgba8(128, 39, 189, 78).into();
    fill_single!(transparent, &transparent_solid);

    let opaque_solid: Paint = AlphaColor::from_rgba8(128, 39, 189, 255).into();
    fill_single!(opaque, &opaque_solid);

    let gradient: Paint = LinearGradient {
        x0: 0.0,
        x1: 256.0,
        stops: vec![
            Stop {
                offset: 0.0,
                color: GREEN,
            },
            Stop {
                offset: 1.0,
                color: BLUE,
            },
        ],
    }
    .into();
    fill_single!(gradient, &gradient);
}
