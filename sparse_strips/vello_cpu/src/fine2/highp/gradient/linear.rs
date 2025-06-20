use core::marker::PhantomData;
use vello_common::encode::LinearKind;
use vello_common::fearless_simd::{f32x16, f32x4, f32x8, Simd};
use crate::fine2::highp::gradient::SimdGradientKind;

#[derive(Debug)]
pub struct SimdLinearKind<S: Simd> {
    phantom_data: PhantomData<S>,
}

impl<S: Simd> From<LinearKind> for SimdLinearKind<S> {
    fn from(_: LinearKind) -> Self {
        Self {
            phantom_data: PhantomData::default(),
        }
    }
}

impl<S: Simd> SimdGradientKind<S> for SimdLinearKind<S> {
    fn cur_pos(&self, x_pos: f32x4<S>, _: f32x4<S>) -> f32x4<S> {
        x_pos
    }
}
