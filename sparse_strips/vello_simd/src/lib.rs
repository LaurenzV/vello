#![allow(non_camel_case_types)]

pub mod neon;
pub mod scalar;

use std::fmt::Debug;
use std::ops::{Add, Mul, Sub};

trait Simd<const C: usize> {}

trait Base:
    Sized + Copy + Add<Self, Output = Self> + Mul<Self, Output = Self> + Sub<Self, Output = Self>
{
}

pub trait Narrowed<const C: usize, F: Scalar>: Base {
    fn load(src: &[F; C]) -> Self;
    fn load_4(src: &[F; 4]) -> Self;
    fn splat(value: F) -> Self;
    fn store(self, dest: &mut [F; C]);

    fn zero() -> Self {
        Self::splat(F::ZERO)
    }

    fn mid() -> Self {
        Self::splat(F::MID)
    }

    fn one() -> Self {
        Self::splat(F::ONE)
    }
}

pub trait Scalar {
    const ZERO: Self;
    const MID: Self;
    const ONE: Self;
}

pub trait Widened<const C: usize, F: Scalar, N: Narrowed<C, F>> {}

pub trait Float<const C: usize>: Narrowed<C, f32> {}

pub trait Integer<const C: usize>: Narrowed<C, u8> {}
