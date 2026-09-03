use alloc::vec;
use alloc::vec::Vec;

// Each entry is round(real BT.601 limited-range coefficient * 65536).
pub(crate) const BT601_COEFFICIENTS: [i32; 9] = [
    16829, 33039, 6416, -9714, -19071, 28784, 28784, -24103, -4681,
];

pub(crate) struct YuvPlanes {
    pub(crate) y: Vec<u8>,
    pub(crate) u: Vec<u8>,
    pub(crate) v: Vec<u8>,
    pub(crate) y_stride: usize,
    pub(crate) chroma_stride: usize,
}

pub(crate) fn convert(
    pixels: &[u8],
    width: usize,
    height: usize,
    bytes_per_pixel: usize,
) -> YuvPlanes {
    let macroblock_columns = width.div_ceil(16);
    let macroblock_rows = height.div_ceil(16);
    let y_stride = macroblock_columns * 16;
    let y_height = macroblock_rows * 16;
    let chroma_stride = macroblock_columns * 8;
    let chroma_height = macroblock_rows * 8;
    let chroma_width = width.div_ceil(2);
    let visible_chroma_height = height.div_ceil(2);

    let mut y_plane = vec![0; y_stride * y_height];
    let mut u_plane = vec![0; chroma_stride * chroma_height];
    let mut v_plane = vec![0; chroma_stride * chroma_height];

    for row in 0..height {
        for column in 0..width {
            let pixel = (row * width + column) * bytes_per_pixel;
            y_plane[row * y_stride + column] =
                convert_y(pixels[pixel], pixels[pixel + 1], pixels[pixel + 2]);
        }
        let edge = y_plane[row * y_stride + width - 1];
        y_plane[row * y_stride + width..(row + 1) * y_stride].fill(edge);
    }
    repeat_last_row(&mut y_plane, y_stride, height, y_height);

    for row in 0..visible_chroma_height {
        let source_y = row * 2;
        let next_y = (source_y + 1).min(height - 1);
        for column in 0..chroma_width {
            let source_x = column * 2;
            let next_x = (source_x + 1).min(width - 1);
            let positions = [
                (source_y * width + source_x) * bytes_per_pixel,
                (source_y * width + next_x) * bytes_per_pixel,
                (next_y * width + source_x) * bytes_per_pixel,
                (next_y * width + next_x) * bytes_per_pixel,
            ];
            let mut u_sum = 0u32;
            let mut v_sum = 0u32;
            for pixel in positions {
                u_sum += u32::from(convert_u(
                    pixels[pixel],
                    pixels[pixel + 1],
                    pixels[pixel + 2],
                ));
                v_sum += u32::from(convert_v(
                    pixels[pixel],
                    pixels[pixel + 1],
                    pixels[pixel + 2],
                ));
            }
            u_plane[row * chroma_stride + column] = ((u_sum + 2) >> 2) as u8;
            v_plane[row * chroma_stride + column] = ((v_sum + 2) >> 2) as u8;
        }
        let u_edge = u_plane[row * chroma_stride + chroma_width - 1];
        let v_edge = v_plane[row * chroma_stride + chroma_width - 1];
        u_plane[row * chroma_stride + chroma_width..(row + 1) * chroma_stride].fill(u_edge);
        v_plane[row * chroma_stride + chroma_width..(row + 1) * chroma_stride].fill(v_edge);
    }
    repeat_last_row(
        &mut u_plane,
        chroma_stride,
        visible_chroma_height,
        chroma_height,
    );
    repeat_last_row(
        &mut v_plane,
        chroma_stride,
        visible_chroma_height,
        chroma_height,
    );

    YuvPlanes {
        y: y_plane,
        u: u_plane,
        v: v_plane,
        y_stride,
        chroma_stride,
    }
}

fn repeat_last_row(plane: &mut [u8], stride: usize, visible_height: usize, height: usize) {
    for row in visible_height..height {
        let source = (visible_height - 1) * stride;
        plane.copy_within(source..source + stride, row * stride);
    }
}

fn convert_y(red: u8, green: u8, blue: u8) -> u8 {
    convert_channel(red, green, blue, &BT601_COEFFICIENTS[..3], 16)
}

fn convert_u(red: u8, green: u8, blue: u8) -> u8 {
    convert_channel(red, green, blue, &BT601_COEFFICIENTS[3..6], 128)
}

fn convert_v(red: u8, green: u8, blue: u8) -> u8 {
    convert_channel(red, green, blue, &BT601_COEFFICIENTS[6..], 128)
}

fn convert_channel(red: u8, green: u8, blue: u8, factors: &[i32], offset: i32) -> u8 {
    let value = factors[0] * i32::from(red)
        + factors[1] * i32::from(green)
        + factors[2] * i32::from(blue)
        + 32768;
    ((value >> 16) + offset) as u8
}

#[cfg(test)]
mod tests {
    use super::convert;

    #[test]
    fn black_and_white_map_to_the_limited_range_endpoints() {
        let planes = convert(&[0, 0, 0, 255, 255, 255], 2, 1, 3);
        assert_eq!(&planes.y[..2], &[16, 235]);
        assert_eq!((planes.u[0], planes.v[0]), (128, 128));
    }

    #[test]
    fn odd_edges_repeat_before_chroma_is_averaged_and_padding_is_added() {
        let pixels = [255, 0, 0, 0, 255, 0, 0, 0, 255];
        let planes = convert(&pixels, 3, 1, 3);
        assert_eq!(&planes.y[..4], &[81, 145, 41, 41]);
        assert_eq!(&planes.u[..3], &[72, 240, 240]);
        assert_eq!(&planes.v[..3], &[137, 110, 110]);
    }
}
