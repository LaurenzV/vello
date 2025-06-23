// mod blend;
// mod clear;
pub(crate) mod fill;
// mod image;
mod gradient;
mod rounded_blurred_rect;
mod strip;

// pub use blend::*;
// pub use clear::*;
pub use fill::*;
pub use gradient::*;
pub use rounded_blurred_rect::*;
pub use strip::*;
use vello_common::peniko::{BlendMode, Compose, Mix};

// #[vello_bench]
// pub fn pack<F: Type>(b: &mut Bencher<'_>, fine: &mut Fine<F>) {
//     let mut buf = vec![0; SCRATCH_BUF_SIZE];
//
//     b.iter(|| {
//         fine.pack(&mut buf);
//         std::hint::black_box(&buf);
//     });
// }

pub(crate) fn default_blend() -> BlendMode {
    BlendMode::new(Mix::Normal, Compose::SrcOver)
}
