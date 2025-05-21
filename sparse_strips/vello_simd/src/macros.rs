// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// Implement the basic arithmetic operations for structs that are based on two other
/// numerical types already implementing them.
#[macro_export]
macro_rules! arith_ops {
    ($name:ident) => {
        impl core::ops::Add for $name {
            type Output = Self;

            #[inline(always)]
            fn add(mut self, rhs: Self) -> Self::Output {
                self.0 = self.0 + rhs.0;
                self.1 = self.1 + rhs.1;

                self
            }
        }

        impl core::ops::Mul for $name {
            type Output = Self;

            #[inline(always)]
            fn mul(mut self, rhs: Self) -> Self::Output {
                self.0 = self.0 * rhs.0;
                self.1 = self.1 * rhs.1;

                self
            }
        }

        impl core::ops::Sub for $name {
            type Output = Self;

            #[inline(always)]
            fn sub(mut self, rhs: Self) -> Self::Output {
                self.0 = self.0 - rhs.0;
                self.1 = self.1 - rhs.1;

                self
            }
        }
    };
}
