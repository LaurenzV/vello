#![allow(non_camel_case_types)]
#![allow(missing_docs)]

pub mod neon;
pub mod scalar;
mod util;

use std::ops::{Add, Mul, Sub};

pub trait Simd<const C: usize> {}

pub trait Base:
    Sized + Copy + Add<Self, Output = Self> + Mul<Self, Output = Self> + Sub<Self, Output = Self>
{
}

pub trait Narrowed<const C: usize, F: Scalar>: Base {
    type Widened: Widened<C, F, Self>;

    fn load(src: &[F; C]) -> Self;
    fn load_4(src: &[F; 4]) -> Self;
    fn splat(value: F) -> Self;
    fn store(self, dest: &mut [F; C]);
    fn widen(self) -> Self::Widened;

    #[inline(always)]
    fn zero() -> Self {
        Self::splat(F::ZERO)
    }

    #[inline(always)]
    fn mid() -> Self {
        Self::splat(F::MID)
    }

    #[inline(always)]
    fn one() -> Self {
        Self::splat(F::ONE)
    }

    #[inline(always)]
    fn one_minus(self) -> Self {
        Self::one() - self
    }

    #[inline(always)]
    fn normalized_mul(self, other: Self) -> Self {
        (self.widen() * other.widen()).normalize().narrow()
    }
}

pub trait Scalar: Base {
    const ZERO: Self;
    const MID: Self;
    const ONE: Self;
}

pub trait Widened<const C: usize, F: Scalar, N: Narrowed<C, F>>: Base {
    fn narrow(self) -> N;
    fn normalize(self) -> Self;
}

pub trait Float<const C: usize>: Narrowed<C, f32> {}

pub trait Integer<const C: usize>: Narrowed<C, u8> {}
