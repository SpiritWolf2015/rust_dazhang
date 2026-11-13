//! `render_headless_rgba` 离屏管线的单元自测（**合成顶点流**）。
//!
//! 前几轮反复黑屏 / 错位的根因之一是「shader / GPU 呈现只能在真窗口人工
//! 观察，CLI 侧无法验证」。本测试用 [`render_headless_rgba`] 在 headless
//! 下用真 adapter 离屏渲染并读回像素，验证**管线本身**：顶点流能上屏、
//! 非黑屏、逐字节确定、空流返回纯背景。这里用的是 NDC 合成三角形，
//! **不证明真实 WC3 模型上屏**——真实模型的证明在
//! `w3-client/tests/real_map_e2e.rs` 的 `real_gltf_models_render_on_screen_not_black`
//! （A/B 对照：仅地形 vs 地形+真实模型，断言模型显著改变像素）。
//!
//! 无 GPU 适配器（CI / headless 环境）时以英文 `key=value` 明确标注跳过，
//! MUST NOT 冒充通过、MUST NOT 静默改判。唯一例外是空顶点流测试：它验证
//! `render_headless_rgba` 的 CPU 早退路径（不建 GPU 资源、返回背景常量），
//! 本就不依赖 adapter，因此无 adapter 时也如实通过（非 GPU 断言）。

use w3_render::{CLEAR_COLOR_SRGB, RenderError, SceneVertex, render_headless_rgba};

/// 覆盖视口中心的大三角形（NDC 内、CCW 正面），用 identity MVP 直接可见。
fn screen_triangle(color: [f32; 4]) -> Vec<SceneVertex> {
    let normal = [0.0, 0.0, 1.0];
    let positions = [[-0.85, -0.85, 0.5], [0.85, -0.85, 0.5], [0.0, 0.85, 0.5]];
    positions
        .iter()
        .map(|&position| SceneVertex::colored(position, normal, color, -1.0))
        .collect()
}

/// 统计 RGBA 里「明显亮于 sRGB 背景且偏目标色」的像素数。
/// 阈值 90 高于 Clear 背景 `CLEAR_COLOR_SRGB` 的最高通道 69，避免把背景误判
/// 为亮像素；红色合成三角形回转后约 (243,123,123) 仍稳定高于阈值。
fn bright_pixels(rgba: &[u8]) -> usize {
    rgba.chunks_exact(4)
        .filter(|p| p[0] > 90 || p[1] > 90 || p[2] > 90)
        .count()
}

#[test]
fn headless_render_produces_nonblack_vertex_pixels() {
    let vertices = screen_triangle([0.9, 0.2, 0.2, 1.0]);
    let view_proj = glam::Mat4::IDENTITY;
    let rgba = match render_headless_rgba(&vertices, &view_proj, 128, 128) {
        Ok(rgba) => rgba,
        Err(RenderError::GpuUnavailable(reason)) => {
            eprintln!("skip=gpu_headless reason=no_adapter detail={reason}");
            return;
        }
        Err(error) => panic!("headless render failed: {error}"),
    };
    assert_eq!(rgba.len(), 128 * 128 * 4, "RGBA 长度应为 w*h*4");
    let lit = bright_pixels(&rgba);
    assert!(
        lit > 1000,
        "合成三角形颜色应占据大量像素，实际亮像素 {lit}（疑似黑屏 / 未上屏）"
    );
    // 有一批像素明显偏红（顶点色），而非背景深蓝或纯灰。
    let redish = rgba
        .chunks_exact(4)
        .filter(|p| p[0] > p[2].saturating_add(30))
        .count();
    assert!(redish > 1000, "应出现顶点色像素，实际 {redish}");
}

#[test]
fn headless_render_is_deterministic() {
    let vertices = screen_triangle([0.2, 0.7, 0.3, 1.0]);
    let view_proj = glam::Mat4::IDENTITY;
    let a = match render_headless_rgba(&vertices, &view_proj, 64, 64) {
        Ok(v) => v,
        Err(RenderError::GpuUnavailable(reason)) => {
            eprintln!("skip=gpu_headless_determinism reason=no_adapter detail={reason}");
            return;
        }
        Err(error) => panic!("headless render failed: {error}"),
    };
    let b = render_headless_rgba(&vertices, &view_proj, 64, 64).expect("second render");
    assert_eq!(a, b, "同一顶点流两次离屏渲染必须逐字节一致");
}

#[test]
fn empty_vertex_stream_yields_clear_color_only() {
    let view_proj = glam::Mat4::IDENTITY;
    // CPU 早退路径不依赖 adapter：无 GPU 也应如实返回正确的 sRGB 背景常量。
    let rgba = render_headless_rgba(&[], &view_proj, 32, 32).expect("empty stream never needs GPU");
    assert_eq!(rgba.len(), 32 * 32 * 4);
    // 精确断言背景色（有判别力）：每个像素都等于 sRGB 回转后的 Clear 背景，
    // 而非旧的 linear 直写值 [5,8,15]，也不是黑屏 / 任意纯色。
    assert!(
        rgba.chunks_exact(4).all(|p| p == CLEAR_COLOR_SRGB),
        "空场景每像素应恰为 CLEAR_COLOR_SRGB={:?}",
        CLEAR_COLOR_SRGB
    );
    assert_eq!(bright_pixels(&rgba), 0, "空场景不应出现模型颜色");
}
