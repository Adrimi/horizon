struct Hud { size: vec4<f32> }
@group(0) @binding(0) var<uniform> hud: Hud;
@group(0) @binding(1) var atlas: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;
struct Out {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}
@vertex fn vs_main(@location(0) pos: vec2<f32>, @location(1) uv: vec2<f32>, @location(2) color: vec4<f32>, @location(3) depth: f32) -> Out {
    let pixels = pos * hud.size.z;
    var out: Out;
    out.clip = vec4(pixels.x / hud.size.x * 2.0 - 1.0, 1.0 - pixels.y / hud.size.y * 2.0, depth, 1.0);
    out.uv = uv;
    out.color = color;
    return out;
}
@fragment fn fs_main(in: Out) -> @location(0) vec4<f32> {
    // u >= 4: the picture is scaled smoothly (bilinear, API 0.1.31).
    if in.uv.x >= 4.0 {
        let size = vec2<f32>(textureDimensions(atlas));
        let p = (in.uv - vec2(4.0, 0.0)) * size - vec2(0.5);
        let base = floor(p);
        let f = p - base;
        let top = vec2<i32>(base);
        let hi = vec2<i32>(size) - vec2(1);
        let a = textureLoad(atlas, clamp(top, vec2(0), hi), 0);
        let b = textureLoad(atlas, clamp(top + vec2(1, 0), vec2(0), hi), 0);
        let c = textureLoad(atlas, clamp(top + vec2(0, 1), vec2(0), hi), 0);
        let d = textureLoad(atlas, clamp(top + vec2(1, 1), vec2(0), hi), 0);
        let mixed = mix(mix(a, b, f.x), mix(c, d, f.x), f.y) * in.color;
        if mixed.a < 0.004 { discard; }
        return mixed;
    }
    // u >= 2: flat vertex colour masked by the texture's alpha (enchantment glint).
    if in.uv.x >= 2.0 {
        let m = textureSample(atlas, atlas_sampler, in.uv - vec2(2.0, 0.0)).a * in.color.a;
        if m < 0.004 { discard; }
        return vec4(in.color.rgb, m);
    }
    let c = textureSample(atlas, atlas_sampler, in.uv) * in.color;
    if c.a < 0.004 { discard; }
    return c;
}
@fragment fn fs_crosshair(in: Out) -> @location(0) vec4<f32> {
    let c = textureSample(atlas, atlas_sampler, in.uv) * in.color;
    if c.a == 0.0 { discard; }
    return c;
}
