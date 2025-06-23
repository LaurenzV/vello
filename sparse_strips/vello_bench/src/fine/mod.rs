mod blend;
// mod clear;
pub(crate) mod fill;
mod image;
mod gradient;
mod rounded_blurred_rect;
mod strip;
mod pack;

pub use blend::*;
// pub use clear::*;
pub use fill::*;
pub use gradient::*;
pub use rounded_blurred_rect::*;
pub use strip::*;
pub use pack::*;
pub use image::*;
use vello_common::peniko::{BlendMode, Compose, Mix};



pub(crate) fn default_blend() -> BlendMode {
    BlendMode::new(Mix::Normal, Compose::SrcOver)
}
