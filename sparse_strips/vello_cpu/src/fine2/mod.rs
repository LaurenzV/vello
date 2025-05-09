use crate::fine::ScratchBuf;
use crate::Paint;

pub const HEIGHT: usize = 4;
pub const WIDETILE_WIDTH: usize = 256;
const COLOR_COMPONENTS: usize = 4;
const TILE_HEIGHT_COMPONENTS: usize = HEIGHT * COLOR_COMPONENTS;
#[doc(hidden)]
pub const SCRATCH_BUF_SIZE: usize =
    WIDETILE_WIDTH * HEIGHT * COLOR_COMPONENTS;

pub fn opaque(
    blend_buf: &mut [u8],
    color: &[u8; 4]
) {
    let blend_buf = &mut blend_buf[0..][..1024];

    
    for t in blend_buf.chunks_exact_mut(COLOR_COMPONENTS) {
        t.copy_from_slice(color);
    }
}

pub struct Fine {
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) wide_coords: (u16, u16),
    pub(crate) blend_buf: Vec<ScratchBuf<u8>>,
    pub(crate) color_buf: ScratchBuf<u8>,
}

impl Fine {
    /// Create a new fine rasterizer.
    pub fn new(width: u16, height: u16) -> Self {
        let blend_buf = [0; SCRATCH_BUF_SIZE];
        let color_buf = [0; SCRATCH_BUF_SIZE];

        Self {
            width,
            height,
            wide_coords: (0, 0),
            blend_buf: vec![blend_buf],
            color_buf,
        }
    }

    /// Fill at a given x and with a width using the given paint.
    pub fn fill(
        &mut self,
        x: usize,
        width: usize,
        fill: &Paint,
    ) {
        let blend_buf = &mut self.blend_buf.last_mut().unwrap()[x * TILE_HEIGHT_COMPONENTS..]
            [..TILE_HEIGHT_COMPONENTS * width];
        let color_buf =
            &mut self.color_buf[x * TILE_HEIGHT_COMPONENTS..][..TILE_HEIGHT_COMPONENTS * width];

        let start_x = self.wide_coords.0 * WIDETILE_WIDTH as u16 + x as u16;
        let start_y = self.wide_coords.1 * HEIGHT as u16;

        match fill {
            Paint::Solid(color) => {
                let color = color.as_premul_rgba8().to_u8_array();

                // If color is completely opaque we can just memcopy the colors.
                if color[3] == 255  {
                    for t in blend_buf.chunks_exact_mut(COLOR_COMPONENTS) {
                        t.copy_from_slice(&color);
                    }

                    return;
                }
            }
            _ => unreachable!()
        }
    }
}