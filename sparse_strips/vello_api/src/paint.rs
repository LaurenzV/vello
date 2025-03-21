// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Types for paints.

use peniko::color::{AlphaColor, Srgb};
use peniko::Extend;
use crate::kurbo::Point;

/// A color stop.
#[derive(Debug, Clone, Copy)]
pub struct Stop {
    /// The normalized offset of the stop.
    pub offset: f32,
    /// The color of the stop.
    pub color: AlphaColor<Srgb>,
}

#[derive(Debug, Clone)]
pub struct LinearGradient {
    /// The x coordinate of the first point.
    pub p0: Point,
    /// The x coordinate of the second point.
    pub p1: Point,
    /// The color stops of the linear gradient.
    pub stops: Vec<Stop>,
    pub extend: Extend
}

// TODO: This will probably turn into a generic type where
// vello-hybrid and vello-cpu provide their own instantiations for
// a `Pattern` type.
/// A paint used for filling or stroking paths.
#[derive(Debug, Clone)]
pub enum Paint {
    /// A solid color.
    Solid(AlphaColor<Srgb>),
    /// A gradient.
    LinearGradient(LinearGradient),
    /// A pattern.
    Pattern(()),
}

impl From<AlphaColor<Srgb>> for Paint {
    fn from(value: AlphaColor<Srgb>) -> Self {
        Self::Solid(value)
    }
}

impl From<LinearGradient> for Paint {
    fn from(value: LinearGradient) -> Self {
        Self::LinearGradient(value)
    }
}