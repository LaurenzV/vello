#![allow(non_camel_case_types)]

pub mod scalar;
pub mod neon;

use std::fmt::Debug;
use std::ops::{Add, Mul, Sub};

trait Simd<const C: usize> {
    
}

trait Numerical: Sized + Copy
+ Add<Self, Output = Self>
+ Mul<Self, Output = Self>
+ Sub<Self, Output = Self>
+ Debug {}

trait Float<const C: usize> {
    fn splat(value: f32) -> Self;
    fn store(self, dest: &mut [f32; C]);
    fn load(src: &[f32; C]) -> Self;
}

trait Integer<const C: usize> {

}