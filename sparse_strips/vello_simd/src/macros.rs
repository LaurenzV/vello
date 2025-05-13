#[macro_export]
macro_rules! arith_ops {
    ($name:ident) => {
        impl Add for $name {
            type Output = Self;
        
            #[inline(always)]
            fn add(mut self, rhs: Self) -> Self::Output {
                self.0 = self.0 + rhs.0;
                self.1 = self.1 + rhs.1;
        
                self
            }
        }
        
        impl Mul for $name {
            type Output = Self;
        
            #[inline(always)]
            fn mul(mut self, rhs: Self) -> Self::Output {
                self.0 = self.0 * rhs.0;
                self.1 = self.1 * rhs.1;
        
                self
            }
        }
        
        impl Sub for $name {
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