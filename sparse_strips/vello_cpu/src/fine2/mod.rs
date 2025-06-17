mod highp;

use vello_common::coarse::WideTile;
use vello_common::tile::Tile;
use crate::RenderMode;

pub(crate) const COLOR_COMPONENTS: usize = 4;
pub(crate) const TILE_HEIGHT_COMPONENTS: usize = Tile::HEIGHT as usize * COLOR_COMPONENTS;
#[doc(hidden)]
pub const SCRATCH_BUF_SIZE: usize =
    WideTile::WIDTH as usize * Tile::HEIGHT as usize * COLOR_COMPONENTS;

pub type ScratchBuf<F> = [F; SCRATCH_BUF_SIZE];

#[derive(Debug)]
struct BuffersContainer<T> {
    pub(crate) blend_buf: Vec<ScratchBuf<T>>,
    pub(crate) paint_buf: ScratchBuf<T>,
}

impl Default for BuffersContainer<u8> {
    fn default() -> Self {
        Self {
            blend_buf: vec![[0; SCRATCH_BUF_SIZE]],
            paint_buf: [0; SCRATCH_BUF_SIZE],
        }
    }
}

impl Default for BuffersContainer<f32> {
    fn default() -> Self {
        Self {
            blend_buf: vec![[0.0; SCRATCH_BUF_SIZE]],
            paint_buf: [0.0; SCRATCH_BUF_SIZE],
        }
    }
}

#[derive(Debug)]
pub(crate) enum Buffers {
    U8(BuffersContainer<u8>),
    F32(BuffersContainer<f32>),
}

impl Buffers {
    fn new(render_mode: RenderMode) -> Self {
        match render_mode {
            RenderMode::OptimizeSpeed => Buffers::U8(Default::default()),
            RenderMode::OptimizeQuality => Buffers::F32(Default::default()),
        }
    }
}

#[derive(Debug)]
#[doc(hidden)]
/// This is an internal struct, do not access directly.
pub struct Fine {
    pub(crate) wide_coords: (u16, u16),
    pub(crate) buffers: Buffers
}

impl Fine {
    /// Create a new fine rasterizer.
    pub fn new(render_mode: RenderMode) -> Self {
        Self {
            wide_coords: (0, 0),
            buffers: Buffers::new(render_mode),
        }
    }

    /// Set the coordinates of the current wide tile that is being processed (in tile units).
    pub fn set_coords(&mut self, x: u16, y: u16) {
        self.wide_coords = (x, y);
    }
}