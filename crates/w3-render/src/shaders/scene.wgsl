// 场景着色器：地形与单位标记共用一条管线（CPU 端已展开为非索引三角形）。
// MVP 由 uniform 提供；颜色按顶点属性（地表 / 队伍色）区分。

struct Globals {
    view_proj : mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> globals : Globals;

struct VertexInput {
    @location(0) position : vec3<f32>,
    @location(1) normal   : vec3<f32>,
    @location(2) color    : vec4<f32>,
    @location(3) surface  : f32,
};

struct VertexOutput {
    @builtin(position) clip   : vec4<f32>,
    @location(0)       color  : vec4<f32>,
    @location(1)       normal : vec3<f32>,
};

@vertex
fn vs_main(input : VertexInput) -> VertexOutput {
    var output : VertexOutput;
    output.clip = globals.view_proj * vec4<f32>(input.position, 1.0);
    output.color = input.color;
    output.normal = input.normal;
    return output;
}

@fragment
fn fs_main(input : VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3<f32>(0.4, 1.0, 0.3));
    let diffuse = 0.45 + 0.55 * max(dot(normalize(input.normal), light_dir), 0.0);
    return vec4<f32>(input.color.rgb * diffuse, input.color.a);
}
