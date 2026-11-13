// 场景着色器：地形（CPU 顶点色）与模型（GPU 纹理数组采样）共用一条管线，
// CPU 端已展开为非索引三角形。顶点 tex_mode 决定片元走顶点色还是纹理采样：
//   0 = 顶点色；1 = 纹理不透明；2 = 纹理 + alpha test（树叶 / 旗帜镂空）。
// 纹理为 texture_2d_array，每层一张 256×256 RGBA8（地表 tileset / 模型贴图）。
// 用 textureSampleLevel（显式 LOD）而非 textureSample：前者不依赖导数，可安全
// 用于含 discard 的分支，避免 WGSL 非均匀控制流约束。

struct Globals {
    view_proj : mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> globals : Globals;
@group(0) @binding(1) var textures : texture_2d_array<f32>;
@group(0) @binding(2) var tex_sampler : sampler;

struct VertexInput {
    @location(0) position : vec3<f32>,
    @location(1) normal   : vec3<f32>,
    @location(2) color    : vec4<f32>,
    @location(3) uv       : vec2<f32>,
    @location(4) tex_layer: f32,
    @location(5) tex_mode : f32,
    @location(6) surface  : f32,
};

struct VertexOutput {
    @builtin(position) clip   : vec4<f32>,
    @location(0)       color  : vec4<f32>,
    @location(1)       normal : vec3<f32>,
    @location(2)       uv     : vec2<f32>,
    @location(3) @interpolate(flat) tex_layer : i32,
    @location(4) @interpolate(flat) tex_mode  : i32,
};

@vertex
fn vs_main(input : VertexInput) -> VertexOutput {
    var output : VertexOutput;
    output.clip = globals.view_proj * vec4<f32>(input.position, 1.0);
    output.color = input.color;
    output.normal = input.normal;
    output.uv = input.uv;
    output.tex_layer = i32(input.tex_layer);
    output.tex_mode = i32(input.tex_mode);
    return output;
}

@fragment
fn fs_main(input : VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3<f32>(0.4, 1.0, 0.3));
    let diffuse = 0.45 + 0.55 * max(dot(normalize(input.normal), light_dir), 0.0);

    var albedo = input.color;
    // 纹理采样模式（1 或 2）：按层取贴图颜色，乘以顶点调制色。
    if (input.tex_mode > 0 && input.tex_layer >= 0) {
        let sampled = textureSampleLevel(textures, tex_sampler, input.uv, input.tex_layer, 0.0);
        // alpha test：树叶 / 旗帜等 MASK 材质丢弃透明片元，呈现镂空树冠，
        // 而不是实心色块或被误删成细杆。
        if (input.tex_mode > 1 && sampled.a < 0.5) {
            discard;
        }
        albedo = vec4<f32>(input.color.rgb * sampled.rgb, input.color.a * sampled.a);
    }

    return vec4<f32>(albedo.rgb * diffuse, albedo.a);
}
