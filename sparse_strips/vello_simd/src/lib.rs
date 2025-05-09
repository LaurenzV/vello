#![allow(non_camel_case_types)]

pub mod neon;
pub mod scalar;

use std::fmt::Debug;
use std::ops::{Add, Mul, Sub};

trait Simd<const C: usize> {}

pub trait Numerical:
    Sized
    + Copy
    + Add<Self, Output = Self>
    + Mul<Self, Output = Self>
    + Sub<Self, Output = Self>
    + Debug
{
}

pub trait Scalar {
    const ZERO: Self;
    const MID: Self;
    const ONE: Self;
}

pub trait Float<const C: usize>: Numerical {
    fn splat(value: f32) -> Self;
    fn store(self, dest: &mut [f32; C]);
    fn load(src: &[f32; C]) -> Self;
    fn load_4(src: &[f32; 4]) -> Self;
    
    fn zero() -> Self {
        Self::splat(f32::ZERO)
    }
    
    fn mid() -> Self {
        Self::splat(f32::MID)
    }

    fn one() -> Self {
        Self::splat(f32::ONE)
    }
}

pub trait Integer<const C: usize>: Numerical {
    fn splat(value: u8) -> Self;
    fn store(self, dest: &mut [u8; C]);
    fn load(src: &[u8; C]) -> Self;
    fn load_4(src: &[u8; 4]) -> Self;

    fn zero() -> Self {
        Self::splat(u8::ZERO)
    }

    fn mid() -> Self {
        Self::splat(u8::MID)
    }

    fn one() -> Self {
        Self::splat(u8::ONE)
    }
}
