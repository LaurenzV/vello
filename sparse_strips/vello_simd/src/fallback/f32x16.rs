use crate::fallback::f32x8::f32x8;
use crate::fallback::u32x16::u32x16;
use crate::{Base, ColorLike, Float, Type, Widened, fallback};
use std::ops::{Add, Div, Mul, Sub};
use crate::fallback::u32x8::u32x8;

#[derive(Copy, Clone, Debug)]
pub struct f32x16(pub(crate) f32x8, pub(crate) f32x8);

impl Add for f32x16 {
    type Output = Self;

    #[inline(always)]
    fn add(mut self, rhs: Self) -> Self::Output {
        self.0 = self.0 + rhs.0;
        self.1 = self.1 + rhs.1;

        self
    }
}

impl Mul for f32x16 {
    type Output = Self;

    #[inline(always)]
    fn mul(mut self, rhs: Self) -> Self::Output {
        self.0 = self.0 * rhs.0;
        self.1 = self.1 * rhs.1;

        self
    }
}

impl Sub for f32x16 {
    type Output = Self;

    #[inline(always)]
    fn sub(mut self, rhs: Self) -> Self::Output {
        self.0 = self.0 - rhs.0;
        self.1 = self.1 - rhs.1;

        self
    }
}

impl Div for f32x16 {
    type Output = Self;

    #[inline(always)]
    fn div(mut self, rhs: Self) -> Self::Output {
        self.0 = self.0 / rhs.0;
        self.1 = self.1 / rhs.1;

        self
    }
}

impl Base for f32x16 {}

impl Type for f32x16 {
    type Widened = Self;

    const LENGTH: usize = 16;

    #[inline(always)]
    fn load(src: &[f32]) -> Self {
        Self(f32x8::load(&src[0..8]), f32x8::load(&src[8..16]))
    }

    #[inline(always)]
    fn splat_4(src: [f32; 4]) -> Self {
        Self(f32x8::splat_4(src), f32x8::splat_4(src))
    }

    #[inline(always)]
    fn splat(value: f32) -> Self {
        Self(f32x8::splat(value), f32x8::splat(value))
    }

    #[inline(always)]
    fn store(self, dest: &mut [f32]) {
        self.0.store(&mut dest[0..8]);
        self.1.store(&mut dest[8..16]);
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
        Self(
            f32x8::load_alphas(&src[0..2]),
            f32x8::load_alphas(&src[2..4]),
        )
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
        self.0 = self.0.min(other.0);
        self.1 = self.1.min(other.1);

        self
    }

    #[inline(always)]
    fn max(mut self, other: Self) -> Self {
        self.0 = self.0.max(other.0);
        self.1 = self.1.max(other.1);

        self
    }

    #[inline(always)]
    fn from_float(f: &[Self::Float]) -> Self {
        f[0]
    }

    #[inline(always)]
    fn splat_4th_element(self) -> Self {
        Self(self.0.splat_4th_element(), self.1.splat_4th_element())
    }

    #[inline(always)]
    fn load_alphas_f32(src: &[f32]) -> Self {
        Self(
            f32x8::load_alphas_f32(&src[0..2]),
            f32x8::load_alphas_f32(&src[2..4]),
        )
    }

    #[inline(always)]
    fn load_f32_many(src: &[f32]) -> Self {
        Self::load(src)
    }

    const IS_FLOAT: bool = true;
}

impl Widened<f32x16> for f32x16 {
    #[inline(always)]
    fn narrow(self) -> f32x16 {
        self
    }

    #[inline(always)]
    fn normalize(self) -> Self {
        self
    }

    #[inline(always)]
    fn clamp(self) -> Self {
        Self(self.0.clamp(), self.1.clamp())
    }
}

impl Float for f32x16 {
    type Integer = u32x16;

    #[inline(always)]
    fn sqrt(mut self) -> Self {
        self.0 = self.0.sqrt();
        self.1 = self.1.sqrt();

        self
    }

    #[inline(always)]
    fn powf(mut self, exponent: f32) -> Self {
        self.0 = self.0.powf(exponent);
        self.1 = self.1.powf(exponent);

        self
    }

    #[inline(always)]
    fn abs(mut self) -> Self {
        self.0 = self.0.abs();
        self.1 = self.1.abs();

        self
    }

    #[inline(always)]
    fn trunc(mut self) -> Self {
        self.0 = self.0.trunc();
        self.1 = self.1.trunc();

        self
    }

    #[inline(always)]
    fn reinterpret(mut self) -> Self::Integer {
        let a  = self.0.reinterpret();
        let b = self.1.reinterpret();

        u32x16(a, b)
    }
    
    #[inline(always)]
    fn floor(mut self) -> Self {
        self.0 = self.0.floor();
        self.1 = self.1.floor();

        self
    }

    #[inline(always)]
    fn fract(mut self) -> Self {
        self.0 = self.0.fract();
        self.1 = self.1.fract();

        self
    }

    #[inline(always)]
    fn lt(mut self, other: Self) -> Self::Integer {
        u32x16(self.0.lt(other.0), self.1.lt(other.1))
    }

    #[inline(always)]
    fn leq(self, other: Self) -> Self::Integer {
        u32x16(self.0.leq(other.0), self.1.leq(other.1))
    }

    #[inline(always)]
    fn ne(mut self, other: Self) -> Self::Integer {
        u32x16(self.0.ne(other.0), self.1.ne(other.1))
    }

    #[inline(always)]
    fn if_then_else(cond: Self::Integer, if_: Self, else_: Self) -> Self {
        f32x16(
            f32x8::if_then_else(cond.0, if_.0, else_.0),
            f32x8::if_then_else(cond.1, if_.1, else_.1),
        )
    }

    #[inline(always)]
    fn splat_col_pos(
        pos: (f32, f32),
        x_advance: (f32, f32),
        y_advance: (f32, f32),
    ) -> (Self, Self) {
        let (f_x, f_y) = f32x8::splat_col_pos(pos, x_advance, y_advance);
        let (s_x, s_y) = f32x8::splat_col_pos(
            (pos.0 + 2.0 * x_advance.0, pos.1 + 2.0 * x_advance.1),
            x_advance,
            y_advance,
        );

        (Self(f_x, s_x), Self(f_y, s_y))
    }
}
