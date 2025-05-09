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
    let blend_buf = &mut blend_buf[..SCRATCH_BUF_SIZE];
    
    for t in blend_buf.chunks_exact_mut(COLOR_COMPONENTS) {
        t.copy_from_slice(color);
    }
}