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
        ($name:ident, $opaque:expr) => {
            g.bench_function(stringify!($name), |b| {
                b.iter(|| {
                    let mut out = vec![];
                    let mut fine = Fine::new(WideTile::WIDTH, Tile::HEIGHT, &mut out);

                    let mut color = ColorIter::new($opaque);

                    for _ in 0..FILL_ITERS {
                        fine.fill(
                            0,
                            0,
                            WideTile::WIDTH as usize,
                            &color.next().unwrap().into(),
                        );
                    }
                })
            });
        };
    }

    fill_single!(fill_transparent, false);
    fill_single!(fill_opaque, true);

    g.bench_function("fill - gradient", |b| {
        b.iter(|| {
            let mut out = vec![];
            let mut fine = Fine::new(WideTile::WIDTH, Tile::HEIGHT, &mut out);

            let gradient = LinearGradient {
                x1: 0.0,
                x2: 256.0,
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
            };
            let paint: Paint = gradient.into();

            for _ in 0..FILL_ITERS {
                fine.fill(0, 0, WideTile::WIDTH as usize, &paint);
            }
        })
    });
}

const SEED: [u8; 32] = [0; 32];

struct ColorIter {
    opaque: bool,
    rng: StdRng,
}

impl ColorIter {
    fn new(opaque: bool) -> Self {
        Self {
            opaque,
            rng: StdRng::from_seed(SEED),
        }
    }
}

impl Iterator for ColorIter {
    type Item = AlphaColor<Srgb>;

    fn next(&mut self) -> Option<Self::Item> {
        let r = self.rng.random_range(0..=255);
        let g = self.rng.random_range(0..=255);
        let b = self.rng.random_range(0..=255);
        let a = if self.opaque {
            255
        } else {
            self.rng.random_range(0..254)
        };

        Some(AlphaColor::from_rgba8(r, g, b, a))
    }
}
