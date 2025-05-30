use crate::{Base, ColorLike, Float, Type, Widened};
use std::ops::{Add, Div, Mul, Sub};

#[derive(Copy, Clone, Debug)]
pub struct f32x4(pub(crate) [f32; 4]);

impl Add for f32x4 {
    type Output = Self;

    #[inline(always)]
    fn add(mut self, rhs: Self) -> Self::Output {
        for i in 0..4 {
            self.0[i] = self.0[i] + rhs.0[i];
        }

        self
    }
}

impl Mul for f32x4 {
    type Output = Self;

    #[inline(always)]
    fn mul(mut self, rhs: Self) -> Self::Output {
        for i in 0..4 {
            self.0[i] = self.0[i] * rhs.0[i];
        }

        self
    }
}

impl Sub for f32x4 {
    type Output = Self;

    #[inline(always)]
    fn sub(mut self, rhs: Self) -> Self::Output {
        for i in 0..4 {
            self.0[i] = self.0[i] - rhs.0[i];
        }

        self
    }
}

impl Div for f32x4 {
    type Output = Self;

    #[inline(always)]
    fn div(mut self, rhs: Self) -> Self::Output {
        for i in 0..4 {
            self.0[i] = self.0[i] / rhs.0[i];
        }

        self
    }
}

impl Base for f32x4 {}

impl Type for f32x4 {
    type Widened = Self;

    const LENGTH: usize = 4;

    #[inline(always)]
    fn load(src: &[f32]) -> Self {
        let src = [src[0], src[1], src[2], src[3]];
        Self(src)
    }

    #[inline(always)]
    fn splat_4(src: [f32; 4]) -> Self {
        Self(src)
    }

    #[inline(always)]
    fn splat(value: f32) -> Self {
        Self([value; 4])
    }

    #[inline(always)]
    fn store(self, dest: &mut [f32]) {
        dest.copy_from_slice(&self.0)
    }

    #[inline(always)]
    fn widen(self) -> Self::Widened {
        self
    }

    #[inline(always)]
    fn normalized_mul(self, other: Self) -> Self {
        self * other
    }

    #[inline(always)]
    fn normalized_mul_add(self, other1: Self, other2: Self) -> Self {
        self * other1 + other2
    }

    #[inline(always)]
    fn normalized_mul_mul_add(self, other1: Self, other2: Self, other3: Self) -> Self {
        self * other1 + other2 * other3
    }

    #[inline(always)]
    fn normalized_mul_sub(self, other1: Self, other2: Self) -> Self {
        other2 - self * other1
    }

    #[inline(always)]
    fn from_normalized_u8(value: u8) -> Self {
        Self::splat(value as f32 / 255.0)
    }

    type Scalar = f32;

    #[inline(always)]
    fn load_alphas(src: &[u8]) -> Self {
        Self::from_normalized_u8(src[0])
    }

    #[inline(always)]
    fn splat_color<T: ColorLike>(color: T) -> Self {
        Self::splat_4(color.to_rgbf32())
    }

    #[inline(always)]
    fn splat_alpha<T: ColorLike>(color: T) -> Self {
        Self::splat(color.to_rgbf32()[3])
    }

    type Float = Self;

    #[inline(always)]
    fn min(mut self, other: Self) -> Self {
        for i in 0..4 {
            self.0[i] = self.0[i].min(other.0[i]);
        }

        self
    }

    #[inline(always)]
    fn max(mut self, other: Self) -> Self {
        for i in 0..4 {
            self.0[i] = self.0[i].max(other.0[i]);
        }

        self
    }

    #[inline(always)]
    fn from_float(f: &[Self::Float]) -> Self {
        f[0]
    }

    #[inline(always)]
    fn splat_4th_element(self) -> Self {
        Self([self.0[3], self.0[3], self.0[3], self.0[3]])
    }

    #[inline(always)]
    fn load_alphas_f32(src: &[f32]) -> Self {
        Self([src[0], src[0], src[0], src[0]])
    }

    #[inline(always)]
    fn load_f32_many(src: &[f32]) -> Self {
        Self::load(src)
    }

    const IS_FLOAT: bool = true;
}

impl Widened<f32x4> for f32x4 {
    #[inline(always)]
    fn narrow(self) -> f32x4 {
        self
    }

    #[inline(always)]
    fn normalize(self) -> Self {
        self
    }

    #[inline(always)]
    fn clamp(mut self) -> Self {
        for i in 0..4 {
            self.0[i] = self.0[i].clamp(0.0, 1.0);
        }

        self
    }
}

impl Float for f32x4 {
    type Mask = [bool; 4];

    #[inline(always)]
    fn sqrt(self) -> Self {
        f32x4([
            self.0[0].sqrt(),
            self.0[1].sqrt(),
            self.0[2].sqrt(),
            self.0[3].sqrt(),
        ])
    }

    #[inline(always)]
    fn powf(self, exponent: f32) -> Self {
        f32x4([
            self.0[0].powf(exponent),
            self.0[1].powf(exponent),
            self.0[2].powf(exponent),
            self.0[3].powf(exponent),
        ])
    }

    #[inline(always)]
    fn abs(self) -> Self {
        f32x4([
            self.0[0].abs(),
            self.0[1].abs(),
            self.0[2].abs(),
            self.0[3].abs(),
        ])
    }

    #[inline(always)]
    fn floor(self) -> Self {
        f32x4([
            self.0[0].floor(),
            self.0[1].floor(),
            self.0[2].floor(),
            self.0[3].floor(),
        ])
    }

    #[inline(always)]
    fn fract(self) -> Self {
        f32x4([
            self.0[0].fract(),
            self.0[1].fract(),
            self.0[2].fract(),
            self.0[3].fract(),
        ])
    }

    #[inline(always)]
    fn lt(self, other: Self) -> Self::Mask {
        [
            self.0[0] < other.0[0],
            self.0[1] < other.0[1],
            self.0[2] < other.0[2],
            self.0[3] < other.0[3],
        ]
    }

    #[inline(always)]
    fn ne(self, other: Self) -> Self::Mask {
        [
            self.0[0] != other.0[0],
            self.0[1] != other.0[1],
            self.0[2] != other.0[2],
            self.0[3] != other.0[3],
        ]
    }

    #[inline(always)]
    fn if_then_else(cond: Self::Mask, mut f_: Self, else_: Self) -> Self {
        f_.0[0] = if cond[0] { f_.0[0] } else {else_.0[0]};
        f_.0[1] = if cond[1] { f_.0[1] } else {else_.0[1]};
        f_.0[2] = if cond[2] { f_.0[2] } else {else_.0[2]};
        f_.0[3] = if cond[3] { f_.0[3] } else {else_.0[3]};
        
        f_
    }

    #[inline(always)]
    fn splat_col_pos(pos: (f32, f32), _: (f32, f32), y_advance: (f32, f32)) -> (Self, Self) {
        let x_pos = f32x4([
            pos.0,
            pos.0 + y_advance.0,
            pos.0 + y_advance.0 * 2.0,
            pos.0 + y_advance.0 * 3.0,
        ]);

        let y_pos = f32x4([
            pos.1,
            pos.1 + y_advance.1,
            pos.1 + y_advance.1 * 2.0,
            pos.1 + y_advance.1 * 3.0,
        ]);

        (x_pos, y_pos)
    }
}

#[inline(always)]
fn select(cond: bool, left: f32, right: f32) -> f32 {
    if cond { left } else { right }
}
