//! GPU readback 的纯 CPU 辅助：行对齐去除与零依赖 PNG 编码。
//!
//! 本机离线缓存没有 `png` / `image`，这里用未压缩 DEFLATE（stored blocks）
//! 直接写 PNG，避免为一次截图引入庞大的依赖树。PNG 编码是可单元测试的
//! 纯逻辑，不依赖 GPU；GPU 侧的纹理拷贝在 [`super::GpuContext`] 中。

use std::path::Path;

use crate::error::RenderError;

/// CRC-32（IEEE 802.3，逐位实现，数据量小无需查表）。
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// Adler-32（zlib 校验）。
fn adler32(data: &[u8]) -> u32 {
    let mut a = 1u32;
    let mut b = 0u32;
    for &byte in data {
        a = (a + u32::from(byte)) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

fn push_chunk(output: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    output.extend_from_slice(&(data.len() as u32).to_be_bytes());
    output.extend_from_slice(kind);
    output.extend_from_slice(data);
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(kind);
    crc_input.extend_from_slice(data);
    output.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

/// 把 RGBA 字节编码成 8-bit RGBA PNG（filter=0、无隔行、stored DEFLATE）。
pub fn encode_png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, RenderError> {
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| RenderError::Readback("image size overflow".into()))?;
    if rgba.len() != expected {
        return Err(RenderError::Readback(format!(
            "rgba length mismatch: expected={} found={}",
            expected,
            rgba.len()
        )));
    }

    let row_bytes = width as usize * 4;
    let mut raw = Vec::with_capacity((row_bytes + 1) * height as usize);
    for row in 0..height as usize {
        raw.push(0); // PNG filter: None
        raw.extend_from_slice(&rgba[row * row_bytes..(row + 1) * row_bytes]);
    }

    // zlib 头 + 若干 stored DEFLATE 块（每块最多 65535 字节）+ Adler-32。
    let mut idat = vec![0x78, 0x01];
    let mut offset = 0usize;
    while offset < raw.len() {
        let chunk = (raw.len() - offset).min(65535);
        let is_final = offset + chunk == raw.len();
        idat.push(u8::from(is_final)); // BFINAL | BTYPE=00
        let len = chunk as u16;
        idat.extend_from_slice(&len.to_le_bytes());
        idat.extend_from_slice(&(!len).to_le_bytes());
        idat.extend_from_slice(&raw[offset..offset + chunk]);
        offset += chunk;
    }
    idat.extend_from_slice(&adler32(&raw).to_be_bytes());

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]); // bit depth / RGBA / deflate / filter / no interlace

    let mut png = Vec::new();
    png.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);
    push_chunk(&mut png, b"IHDR", &ihdr);
    push_chunk(&mut png, b"IDAT", &idat);
    push_chunk(&mut png, b"IEND", &[]);
    Ok(png)
}

/// 把 RGBA 截图写到本机文件（截图产物留在本机，绝不入库，见 ADR 0001）。
pub fn write_png(path: &Path, rgba: &[u8], width: u32, height: u32) -> Result<(), RenderError> {
    let bytes = encode_png(rgba, width, height)?;
    std::fs::write(path, bytes)?;
    Ok(())
}

/// 去掉 `copy_texture_to_buffer` 的 256 字节行对齐填充，得到紧凑 RGBA。
pub fn remove_row_padding(
    padded: &[u8],
    width: u32,
    height: u32,
    bytes_per_row: u32,
) -> Result<Vec<u8>, RenderError> {
    let row = width as usize * 4;
    let stride = bytes_per_row as usize;
    if stride < row {
        return Err(RenderError::Readback(format!(
            "stride too small: stride={} row={}",
            stride, row
        )));
    }
    let needed = stride
        .checked_mul(height as usize)
        .ok_or_else(|| RenderError::Readback("padded size overflow".into()))?;
    if padded.len() < needed {
        return Err(RenderError::Readback(format!(
            "padded buffer too short: need={} found={}",
            needed,
            padded.len()
        )));
    }
    let mut rgba = Vec::with_capacity(row * height as usize);
    for y in 0..height as usize {
        rgba.extend_from_slice(&padded[y * stride..y * stride + row]);
    }
    Ok(rgba)
}

/// BGRA → RGBA 通道交换（`Bgra8Unorm` 表面 readback 时调用）。
pub fn swap_bgra_to_rgba(rgba: &mut [u8]) {
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_and_adler32_match_known_values() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        assert_eq!(adler32(b"Wikipedia"), 0x11e6_0398);
    }

    #[test]
    fn encode_png_has_valid_structure() {
        let rgba = [255u8, 0, 0, 255, 0, 255, 0, 128]; // 2x1
        let png = encode_png(&rgba, 2, 1).unwrap();
        assert_eq!(&png[..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
        // IHDR 长度与类型、宽高、位深 / 颜色类型。
        assert_eq!(&png[8..12], &[0, 0, 0, 13]);
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(&png[16..20], &2u32.to_be_bytes());
        assert_eq!(&png[20..24], &1u32.to_be_bytes());
        assert_eq!(&png[24..29], &[8, 6, 0, 0, 0]);
        // IDAT 与 IEND chunk 类型均存在。
        assert!(png.windows(4).any(|w| w == b"IDAT"));
        assert!(png.windows(4).any(|w| w == b"IEND"));
    }

    #[test]
    fn encode_png_rejects_wrong_size() {
        assert!(matches!(
            encode_png(&[0u8; 3], 1, 1),
            Err(RenderError::Readback(_))
        ));
    }

    #[test]
    fn row_padding_is_removed() {
        // 宽 1 => 4 字节有效数据，stride 256。
        let mut padded = vec![0u8; 256 * 2];
        padded[0..4].copy_from_slice(&[10, 20, 30, 40]);
        padded[256..260].copy_from_slice(&[50, 60, 70, 80]);
        let rgba = remove_row_padding(&padded, 1, 2, 256).unwrap();
        assert_eq!(rgba, vec![10, 20, 30, 40, 50, 60, 70, 80]);
    }

    #[test]
    fn bgra_swap_exchanges_channels() {
        let mut pixel = [10u8, 20, 30, 40];
        swap_bgra_to_rgba(&mut pixel);
        assert_eq!(pixel, [30, 20, 10, 40]);
    }
}
