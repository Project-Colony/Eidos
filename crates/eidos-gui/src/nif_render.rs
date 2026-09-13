//! Bounded static NIF preview; input is the bundled helper's versioned JSON.
use serde::{
    de::{self, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use std::{
    collections::{HashMap, HashSet},
    fmt,
    marker::PhantomData,
    sync::atomic::{AtomicBool, Ordering},
};

pub(crate) const WIDTH: u32 = 640;
pub(crate) const HEIGHT: u32 = 420;
pub(crate) const MAX_TEXTURE_SIDE: u32 = 4096;
pub(crate) const MAX_TEXTURE_BYTES: usize = 64 * 1024 * 1024;
const MAX_JSON: usize = 64 * 1024 * 1024;
const MAX_VERTICES: usize = 250_000;
const MAX_TRIANGLES: usize = 500_000;
const MAX_MESHES: usize = 1024;
const MAX_PIXEL_WORK: usize = 32_000_000;
const BACKGROUND: [u8; 4] = [24, 27, 34, 255];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Bounds {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Material {
    pub shader: String,
    #[serde(deserialize_with = "limited::<_, _, 16>")]
    pub textures: Vec<String>,
    pub alpha: f32,
    pub double_sided: bool,
    pub alpha_flags: u16,
    pub alpha_threshold: u8,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Mesh {
    pub block: u32,
    pub name: String,
    pub block_type: String,
    #[serde(deserialize_with = "limited::<_, _, MAX_VERTICES>")]
    positions: Vec<[f64; 3]>,
    #[serde(deserialize_with = "limited::<_, _, MAX_VERTICES>")]
    normals: Vec<[f64; 3]>,
    #[serde(deserialize_with = "limited::<_, _, MAX_VERTICES>")]
    uvs: Vec<[f64; 2]>,
    #[serde(deserialize_with = "limited::<_, _, MAX_TRIANGLES>")]
    triangles: Vec<[u32; 3]>,
    pub material: Material,
    #[serde(skip)]
    diffuse_key: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Scene {
    version: u32,
    nif_version: String,
    user_version: u32,
    stream_version: u32,
    pub bounds: Option<Bounds>,
    #[serde(deserialize_with = "meshes")]
    pub meshes: Vec<Mesh>,
    #[serde(deserialize_with = "limited::<_, _, 20000>")]
    pub warnings: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct View {
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
    pub wireframe: bool,
}
impl Default for View {
    fn default() -> Self {
        Self {
            yaw: 0.5,
            pitch: 0.45,
            zoom: 1.0,
            wireframe: false,
        }
    }
}

#[derive(Debug)]
pub(crate) struct Texture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
#[derive(Debug)]
pub(crate) struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

// ponytail: bounded serde sequences reuse the existing JSON parser; no second parser.
fn limited<'de, D, T, const MAX: usize>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Items<T, const MAX: usize>(PhantomData<T>);
    impl<'de, T: Deserialize<'de>, const MAX: usize> Visitor<'de> for Items<T, MAX> {
        type Value = Vec<T>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "at most {MAX} items")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut out = Vec::new();
            while let Some(item) = seq.next_element()? {
                if out.len() == MAX {
                    return Err(de::Error::custom("NIF sequence limit exceeded"));
                }
                out.push(item);
            }
            Ok(out)
        }
    }
    deserializer.deserialize_seq(Items::<T, MAX>(PhantomData))
}

fn meshes<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Mesh>, D::Error> {
    struct Meshes;
    impl<'de> Visitor<'de> for Meshes {
        type Value = Vec<Mesh>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("bounded static meshes")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let (mut vertices, mut triangles) = (0, 0);
            let mut out = Vec::new();
            while let Some(mesh) = seq.next_element::<Mesh>()? {
                // Reject mismatched arrays before retaining another mesh, so
                // malformed normals/UVs cannot bypass the aggregate vertex cap.
                if mesh.positions.is_empty()
                    || mesh.normals.len() != mesh.positions.len()
                    || (!mesh.uvs.is_empty() && mesh.uvs.len() != mesh.positions.len())
                {
                    return Err(de::Error::custom(
                        "Inconsistent NIF vertex/normal/UV counts",
                    ));
                }
                vertices += mesh.positions.len();
                triangles += mesh.triangles.len();
                if out.len() == MAX_MESHES || vertices > MAX_VERTICES || triangles > MAX_TRIANGLES {
                    return Err(de::Error::custom("NIF scene count limit exceeded"));
                }
                out.push(mesh);
            }
            Ok(out)
        }
    }
    deserializer.deserialize_seq(Meshes)
}

pub(crate) fn normalize_texture_path(path: &str) -> Result<String, String> {
    if path.is_empty()
        || path.len() > 4096
        || path.chars().any(char::is_control)
        || path.contains(':')
    {
        return Err("Invalid texture path".into());
    }
    let path = path.replace('\\', "/");
    if path.starts_with('/') || path.split('/').any(|p| p == "..") {
        return Err("Texture path must stay relative to Data".into());
    }
    let normalized = path
        .split('/')
        .filter(|p| !p.is_empty() && *p != ".")
        .collect::<Vec<_>>()
        .join("/")
        .to_ascii_lowercase();
    if normalized.is_empty() {
        return Err("Empty texture path".into());
    }
    Ok(normalized)
}

fn finite(values: &[f64]) -> bool {
    values.iter().all(|v| v.is_finite() && v.abs() <= 1e9)
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}

pub(crate) fn parse(bytes: &[u8]) -> Result<Scene, String> {
    if bytes.len() > MAX_JSON {
        return Err("NIF helper JSON exceeds 64 MiB".into());
    }
    let mut scene: Scene =
        serde_json::from_slice(bytes).map_err(|e| format!("Invalid NIF helper JSON: {e}"))?;
    if scene.version != 1
        || scene.nif_version != "20.2.0.7"
        || scene.user_version != 12
        || ![83, 100].contains(&scene.stream_version)
    {
        return Err("Unsupported NIF helper/version combination".into());
    }
    if scene.warnings.iter().any(|s| s.len() > 4096) {
        return Err("NIF warning string exceeds limit".into());
    }
    let mut actual = Bounds {
        min: [f64::INFINITY; 3],
        max: [f64::NEG_INFINITY; 3],
    };
    let mut vertex_count = 0;
    let mut blocks = HashSet::new();
    for mesh in &mut scene.meshes {
        if mesh.block >= 20000
            || !blocks.insert(mesh.block)
            || mesh.name.len() > 4096
            || mesh.block_type.len() > 128
            || mesh.material.shader.len() > 128
            || !["NiTriShape", "BSTriShape"].contains(&mesh.block_type.as_str())
        {
            return Err("Unsupported mesh block or oversized string".into());
        }
        if mesh.positions.is_empty()
            || mesh.normals.len() != mesh.positions.len()
            || (!mesh.uvs.is_empty() && mesh.uvs.len() != mesh.positions.len())
        {
            return Err("Inconsistent NIF vertex/normal/UV counts".into());
        }
        if !mesh.material.alpha.is_finite()
            || !(0.0..=1.0).contains(&mesh.material.alpha)
            || mesh.material.textures.iter().any(|s| s.len() > 4096)
        {
            return Err("Invalid NIF material".into());
        }
        vertex_count += mesh.positions.len();
        for p in &mesh.positions {
            if !finite(p) {
                return Err("Invalid NIF position".into());
            }
            for (axis, value) in p.iter().enumerate() {
                actual.min[axis] = actual.min[axis].min(*value);
                actual.max[axis] = actual.max[axis].max(*value);
            }
        }
        if mesh
            .normals
            .iter()
            .any(|n| !finite(n) || dot(*n, *n) < 1e-20)
            || mesh.uvs.iter().any(|uv| !finite(uv))
        {
            return Err("Invalid NIF normal or UV".into());
        }
        if mesh
            .triangles
            .iter()
            .flatten()
            .any(|&i| i as usize >= mesh.positions.len())
        {
            return Err("NIF triangle index out of range".into());
        }
        if let Some(path) = mesh.material.textures.first().filter(|s| !s.is_empty()) {
            match normalize_texture_path(path) {
                Ok(key) => mesh.diffuse_key = Some(key),
                Err(e) => scene
                    .warnings
                    .push(format!("Block {} diffuse texture omitted: {e}", mesh.block)),
            }
            if mesh.uvs.is_empty() {
                scene.warnings.push(format!(
                    "Block {} has no UVs; showing neutral material",
                    mesh.block
                ));
            }
        }
        let flags = mesh.material.alpha_flags;
        if mesh.material.alpha < 1.0 && flags & 1 == 0 {
            scene.warnings.push(format!(
                "Block {}: material opacity shown with source-alpha blending",
                mesh.block
            ));
        }
        if flags & 1 != 0 && ((flags >> 1) & 15 != 6 || (flags >> 5) & 15 != 7) {
            scene.warnings.push(format!(
                "Block {}: nonstandard blend factors approximated by source-alpha blending",
                mesh.block
            ));
        }
        if flags & 0xa000 != 0 {
            scene.warnings.push(format!(
                "Block {}: alpha sorter/editor-threshold override is not evaluated",
                mesh.block
            ));
        }
    }
    match (&scene.bounds, vertex_count) {
        (None, 0) => {}
        (Some(bounds), n) if n > 0 && finite(&bounds.min) && finite(&bounds.max) => {
            for axis in 0..3 {
                if bounds.min[axis] != actual.min[axis] || bounds.max[axis] != actual.max[axis] {
                    return Err("NIF bounds do not match returned geometry".into());
                }
            }
            scene.bounds = Some(actual);
        }
        _ => return Err("Missing or inconsistent NIF bounds".into()),
    }
    scene.warnings.push("Static orthographic preview: diffuse/headlight shading only; engine shaders, animation, skinning and normal maps are not reproduced. Transparency uses triangle-center sorting and source-alpha blending in display RGB.".into());
    Ok(scene)
}

#[derive(Clone, Copy)]
struct Vertex {
    screen: [f32; 3],
    uv: [f64; 2],
    light: f32,
}
struct Triangle {
    mesh: usize,
    indices: [u32; 3],
    bbox: [u16; 4],
    area: f64,
    depth: f32,
    order: u32,
}

fn cancelled(cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(Ordering::Relaxed) {
        Err("NIF rendering cancelled".into())
    } else {
        Ok(())
    }
}
fn edge(a: [f32; 3], b: [f32; 3], p: [f32; 3]) -> f64 {
    let (ax, ay, bx, by) = (a[0] as f64, a[1] as f64, b[0] as f64, b[1] as f64);
    // Reversing an edge negates the same coefficients, so neighboring faces
    // cannot both include a shared pixel through independent rounding.
    (ay - by) * p[0] as f64 + (bx - ax) * p[1] as f64 + (ax * by - ay * bx)
}
fn top_left(a: [f32; 3], b: [f32; 3]) -> bool {
    b[1] < a[1] || (b[1] == a[1] && b[0] > a[0])
}
fn blends(material: &Material) -> bool {
    material.alpha_flags & 1 != 0 || material.alpha < 1.0
}

// NiAlphaProperty bit layout/test enums: niftools/nifxml AlphaFlags/TestFunction.
fn alpha_passes(material: &Material, alpha: u8) -> bool {
    if material.alpha_flags & 0x200 == 0 {
        return true;
    }
    let threshold = material.alpha_threshold;
    match (material.alpha_flags >> 10) & 7 {
        0 => true,
        1 => alpha < threshold,
        2 => alpha == threshold,
        3 => alpha <= threshold,
        4 => alpha > threshold,
        5 => alpha != threshold,
        6 => alpha >= threshold,
        _ => false,
    }
}

fn validate_textures(textures: &HashMap<String, Texture>) -> Result<(), String> {
    if textures.len() > MAX_MESHES {
        return Err("NIF texture count exceeds 1024".into());
    }
    let mut total = 0;
    for (key, texture) in textures {
        if normalize_texture_path(key)? != *key {
            return Err("Texture map keys must be normalized".into());
        }
        if texture.width == 0
            || texture.height == 0
            || texture.width > MAX_TEXTURE_SIDE
            || texture.height > MAX_TEXTURE_SIDE
        {
            return Err("NIF texture dimensions must be within 1..4096".into());
        }
        let bytes = texture.width as usize * texture.height as usize * 4;
        if texture.rgba.len() != bytes {
            return Err("NIF texture RGBA length mismatch".into());
        }
        if bytes > MAX_TEXTURE_BYTES - total {
            return Err("NIF textures exceed 64 MiB total".into());
        }
        total += bytes;
    }
    Ok(())
}

fn sample(texture: &Texture, uv: [f64; 2]) -> [u8; 4] {
    // ponytail: nearest repeated sampling; add filtering only if preview quality needs it.
    let x = (uv[0].rem_euclid(1.0) * texture.width as f64) as usize;
    let y = (uv[1].rem_euclid(1.0) * texture.height as f64) as usize;
    let i = (y.min(texture.height as usize - 1) * texture.width as usize
        + x.min(texture.width as usize - 1))
        * 4;
    [
        texture.rgba[i],
        texture.rgba[i + 1],
        texture.rgba[i + 2],
        texture.rgba[i + 3],
    ]
}

pub(crate) fn render(
    scene: &Scene,
    view: View,
    textures: &HashMap<String, Texture>,
    cancel: &AtomicBool,
) -> Result<Frame, String> {
    cancelled(cancel)?;
    if !view.yaw.is_finite() || !view.pitch.is_finite() || !view.zoom.is_finite() {
        return Err("NIF camera values must be finite".into());
    }
    validate_textures(textures)?;
    let bounds = scene
        .bounds
        .as_ref()
        .ok_or("No static NIF geometry is available")?;
    if scene.meshes.iter().all(|m| m.triangles.is_empty()) {
        return Err("No static NIF triangles are available".into());
    }
    let center = std::array::from_fn::<_, 3, _>(|i| (bounds.min[i] + bounds.max[i]) * 0.5);
    let extent = std::array::from_fn::<_, 3, _>(|i| (bounds.max[i] - bounds.min[i]) * 0.5);
    let radius = dot(extent, extent).sqrt();
    let radius = if radius > 0.0 { radius } else { 1.0 };
    let yaw = view.yaw.rem_euclid(std::f32::consts::TAU) as f64;
    let pitch = view
        .pitch
        .clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2) as f64;
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    let right = [cy, sy, 0.0];
    let up = [-sp * sy, sp * cy, cp];
    let front = [cp * sy, -cp * cy, sp];
    let scale = HEIGHT as f64 * 0.425 * view.zoom.clamp(0.1, 10.0) as f64;
    let mut vertices = Vec::with_capacity(scene.meshes.len());
    for mesh in &scene.meshes {
        let mut projected = Vec::with_capacity(mesh.positions.len());
        for (i, p) in mesh.positions.iter().enumerate() {
            if i % 1024 == 0 {
                cancelled(cancel)?;
            }
            let p = std::array::from_fn(|a| (p[a] - center[a]) / radius);
            let normal = mesh.normals[i];
            let cosine = dot(normal, front) / dot(normal, normal).sqrt();
            let light = 0.3
                + 0.7
                    * if mesh.material.double_sided {
                        cosine.abs()
                    } else {
                        cosine.max(0.0)
                    };
            projected.push(Vertex {
                screen: [
                    (WIDTH as f64 * 0.5 + dot(p, right) * scale) as f32,
                    (HEIGHT as f64 * 0.5 - dot(p, up) * scale) as f32,
                    dot(p, front) as f32,
                ],
                uv: mesh.uvs.get(i).copied().unwrap_or_default(),
                light: light as f32,
            });
        }
        vertices.push(projected);
    }
    let (mut opaque, mut transparent) = (Vec::new(), Vec::new());
    let (mut work, mut order) = (0usize, 0u32);
    for (mi, mesh) in scene.meshes.iter().enumerate() {
        for ids in &mesh.triangles {
            if order % 256 == 0 {
                cancelled(cancel)?;
            }
            order += 1;
            let mut ids = *ids;
            let mut p = ids.map(|i| vertices[mi][i as usize].screen);
            let mut area = edge(p[0], p[1], p[2]);
            if area.abs() < 1e-6 || (area > 0.0 && !mesh.material.double_sided) {
                continue;
            }
            if area < 0.0 {
                ids.swap(1, 2);
                p.swap(1, 2);
                area = -area;
            }
            let xmin = p
                .iter()
                .map(|v| v[0])
                .fold(f32::INFINITY, f32::min)
                .floor()
                .clamp(0.0, WIDTH as f32) as u16;
            let xmax = p
                .iter()
                .map(|v| v[0])
                .fold(f32::NEG_INFINITY, f32::max)
                .ceil()
                .clamp(0.0, WIDTH as f32) as u16;
            let ymin = p
                .iter()
                .map(|v| v[1])
                .fold(f32::INFINITY, f32::min)
                .floor()
                .clamp(0.0, HEIGHT as f32) as u16;
            let ymax = p
                .iter()
                .map(|v| v[1])
                .fold(f32::NEG_INFINITY, f32::max)
                .ceil()
                .clamp(0.0, HEIGHT as f32) as u16;
            if xmin == xmax || ymin == ymax {
                continue;
            }
            // ponytail: cap bounding-box work before rasterizing, not elapsed time.
            let pixels = (xmax - xmin) as usize * (ymax - ymin) as usize;
            if pixels > MAX_PIXEL_WORK - work {
                return Err("NIF pixel work limit exceeds 32 million samples".into());
            }
            work += pixels;
            let triangle = Triangle {
                mesh: mi,
                indices: ids,
                bbox: [xmin, xmax, ymin, ymax],
                area,
                depth: (p[0][2] + p[1][2] + p[2][2]) / 3.0,
                order,
            };
            if blends(&mesh.material) {
                transparent.push(triangle);
            } else {
                opaque.push(triangle);
            }
        }
    }
    // ponytail: triangle-center sorting is bounded at 500k triangles; intersecting
    // transparent surfaces need a different transparency algorithm for exactness.
    transparent.sort_unstable_by(|a, b| a.depth.total_cmp(&b.depth).then(a.order.cmp(&b.order)));
    cancelled(cancel)?;
    let mut frame = Frame {
        width: WIDTH,
        height: HEIGHT,
        rgba: vec![0; WIDTH as usize * HEIGHT as usize * 4],
    };
    for pixel in frame.rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&BACKGROUND);
    }
    let mut depth = vec![f32::NEG_INFINITY; WIDTH as usize * HEIGHT as usize];
    for triangle in opaque.iter().chain(&transparent) {
        let mesh = &scene.meshes[triangle.mesh];
        let material = &mesh.material;
        let translucent = blends(material);
        let texture = mesh
            .diffuse_key
            .as_ref()
            .and_then(|k| textures.get(k))
            .filter(|_| !mesh.uvs.is_empty());
        let v = triangle
            .indices
            .map(|i| vertices[triangle.mesh][i as usize]);
        let p = v.map(|v| v.screen);
        let inclusive = [
            top_left(p[1], p[2]),
            top_left(p[2], p[0]),
            top_left(p[0], p[1]),
        ];
        let edge_length = [
            ((p[1][0] - p[2][0]).powi(2) + (p[1][1] - p[2][1]).powi(2)).sqrt(),
            ((p[2][0] - p[0][0]).powi(2) + (p[2][1] - p[0][1]).powi(2)).sqrt(),
            ((p[0][0] - p[1][0]).powi(2) + (p[0][1] - p[1][1]).powi(2)).sqrt(),
        ];
        for y in triangle.bbox[2] as usize..triangle.bbox[3] as usize {
            cancelled(cancel)?;
            for x in triangle.bbox[0] as usize..triangle.bbox[1] as usize {
                let point = [x as f32 + 0.5, y as f32 + 0.5, 0.0];
                let e = [
                    edge(p[1], p[2], point),
                    edge(p[2], p[0], point),
                    edge(p[0], p[1], point),
                ];
                if e.iter()
                    .zip(inclusive)
                    .any(|(&value, include)| value < 0.0 || (value == 0.0 && !include))
                {
                    continue;
                }
                let w = e.map(|e| e / triangle.area);
                let z = (0..3).map(|i| w[i] * p[i][2] as f64).sum::<f64>() as f32;
                let index = y * WIDTH as usize + x;
                if z <= depth[index] {
                    continue;
                }
                let uv = std::array::from_fn(|axis| (0..3).map(|i| w[i] * v[i].uv[axis]).sum());
                let color = texture.map_or([160, 168, 180, 255], |texture| sample(texture, uv));
                let alpha = (color[3] as f32 / 255.0 * material.alpha).clamp(0.0, 1.0);
                if !alpha_passes(material, (alpha * 255.0).round() as u8)
                    || (translucent && alpha == 0.0)
                {
                    continue;
                }
                let boundary = e
                    .iter()
                    .zip(edge_length)
                    .any(|(&distance, length)| distance <= length as f64 * 0.85);
                if view.wireframe && translucent && !boundary {
                    continue;
                }
                let light = (0..3)
                    .map(|i| w[i] * v[i].light as f64)
                    .sum::<f64>()
                    .clamp(0.0, 1.0) as f32;
                for channel in 0..3 {
                    let source = if view.wireframe {
                        if boundary {
                            [220, 230, 245][channel] as f32
                        } else {
                            BACKGROUND[channel] as f32
                        }
                    } else {
                        color[channel] as f32 * light
                    };
                    let dest = &mut frame.rgba[index * 4 + channel];
                    *dest = (if translucent {
                        source * alpha + *dest as f32 * (1.0 - alpha)
                    } else {
                        source
                    })
                    .round()
                    .clamp(0.0, 255.0) as u8;
                }
                if !translucent {
                    depth[index] = z;
                }
            }
        }
    }
    cancelled(cancel)?;
    Ok(frame)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn mesh(z: f64, texture: &str, alpha: f32) -> Value {
        json!({"block": 4, "name":"Triangle", "block_type":"NiTriShape",
            "positions":[[-1.0,-1.0,z],[1.0,-1.0,z],[0.0,1.0,z]],
            "normals":[[0,0,1],[0,0,1],[0,0,1]], "uvs":[[0,0],[0,0],[0,0]], "triangles":[[0,1,2]],
            "material":{"shader":"BSLightingShaderProperty", "textures":[texture], "alpha":alpha,
            "double_sided":false, "alpha_flags":if alpha < 1.0 {237} else {0}, "alpha_threshold":127}})
    }
    fn source(mut meshes: Vec<Value>) -> Value {
        for (i, mesh) in meshes.iter_mut().enumerate() {
            mesh["block"] = json!(i);
        }
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        for mesh in &meshes {
            for p in mesh["positions"].as_array().unwrap() {
                for axis in 0..3 {
                    let v = p[axis].as_f64().unwrap();
                    min[axis] = min[axis].min(v);
                    max[axis] = max[axis].max(v);
                }
            }
        }
        json!({"version":1,"nif_version":"20.2.0.7","user_version":12,"stream_version":83,
            "bounds": if meshes.is_empty() { Value::Null } else { json!({"min":min,"max":max}) }, "meshes":meshes,"warnings":[]})
    }
    fn scene(meshes: Vec<Value>) -> Scene {
        parse(&serde_json::to_vec(&source(meshes)).unwrap()).unwrap()
    }
    fn top() -> View {
        View {
            yaw: 0.0,
            pitch: std::f32::consts::FRAC_PI_2,
            ..View::default()
        }
    }
    fn textures(colors: &[(&str, [u8; 4])]) -> HashMap<String, Texture> {
        colors
            .iter()
            .map(|(key, rgba)| {
                (
                    key.to_string(),
                    Texture {
                        width: 1,
                        height: 1,
                        rgba: rgba.to_vec(),
                    },
                )
            })
            .collect()
    }
    fn pixel(frame: &Frame, x: usize, y: usize) -> [u8; 4] {
        frame.rgba[(y * WIDTH as usize + x) * 4..][..4]
            .try_into()
            .unwrap()
    }

    #[test]
    fn opaque_center_and_background_match_hand_derived_pixels() {
        let scene = scene(vec![mesh(0.0, "blue.dds", 1.0)]);
        let frame = render(
            &scene,
            top(),
            &textures(&[("blue.dds", [0, 0, 255, 255])]),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!((frame.width, frame.height), (640, 420));
        assert_eq!(pixel(&frame, 320, 210), [0, 0, 255, 255]);
        assert_eq!(pixel(&frame, 0, 0), BACKGROUND);
    }

    #[test]
    fn malformed_indices_bounds_and_versions_are_rejected() {
        let input = source(vec![mesh(0.0, "", 1.0)]);
        for (field, value) in [
            ("version", json!(2)),
            ("stream_version", json!(130)),
            ("bounds", json!({"min":[0,0,0],"max":[1,1,1]})),
        ] {
            let mut bad = input.clone();
            bad[field] = value;
            assert!(parse(&serde_json::to_vec(&bad).unwrap()).is_err());
        }
        let mut bad = input;
        bad["meshes"][0]["triangles"] = json!([[0, 1, 9]]);
        assert!(parse(&serde_json::to_vec(&bad).unwrap()).is_err());
    }

    #[test]
    fn native_le_and_sse_fixtures_preserve_world_geometry_and_texture_slots() {
        for (json, kind) in [
            (
                include_bytes!("nif_render_fixtures/le.json").as_slice(),
                "NiTriShape",
            ),
            (
                include_bytes!("nif_render_fixtures/sse.json").as_slice(),
                "BSTriShape",
            ),
        ] {
            let scene = parse(json).unwrap();
            assert_eq!(scene.meshes.len(), 1);
            let mesh = &scene.meshes[0];
            assert_eq!(mesh.block_type, kind);
            assert_eq!(
                mesh.positions,
                [[10.0, 22.0, 30.0], [10.0, 24.0, 30.0], [8.0, 22.0, 30.0]]
            );
            assert_eq!(mesh.triangles, [[0, 1, 2]]);
            assert_eq!(mesh.material.textures[1], "textures\\synthetic\\normal.dds");
            assert_eq!(
                mesh.diffuse_key.as_deref(),
                Some("textures/synthetic/diffuse.dds")
            );
            let frame = render(&scene, top(), &HashMap::new(), &AtomicBool::new(false)).unwrap();
            assert!(frame.rgba.chunks_exact(4).any(|p| p != BACKGROUND));
        }
    }

    #[test]
    fn opaque_occlusion_and_transparent_sorting_match_independent_colors() {
        let maps = textures(&[("red", [255, 0, 0, 255]), ("blue", [0, 0, 255, 255])]);
        for reversed in [false, true] {
            let mut meshes = vec![mesh(0.4, "red", 1.0), mesh(-0.4, "blue", 1.0)];
            if reversed {
                meshes.reverse();
            }
            let frame = render(&scene(meshes), top(), &maps, &AtomicBool::new(false)).unwrap();
            assert_eq!(pixel(&frame, 320, 210), [255, 0, 0, 255]);
            let mut meshes = vec![mesh(0.4, "red", 0.5), mesh(-0.4, "blue", 0.5)];
            if reversed {
                meshes.reverse();
            }
            let frame = render(&scene(meshes), top(), &maps, &AtomicBool::new(false)).unwrap();
            // Blue over [24,27,34], then red, each at one-half opacity.
            assert_eq!(pixel(&frame, 320, 210), [134, 7, 73, 255]);
        }
        let frame = render(
            &scene(vec![mesh(0.4, "red", 1.0), mesh(-0.4, "blue", 0.5)]),
            top(),
            &maps,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(pixel(&frame, 320, 210), [255, 0, 0, 255]);
    }

    #[test]
    fn alpha_test_wrap_and_shared_edges_have_golden_pixels() {
        let expectations = [
            [true, true, true],
            [true, false, false],
            [false, true, false],
            [true, true, false],
            [false, false, true],
            [true, false, true],
            [false, true, true],
            [false, false, false],
        ];
        for (mode, expected) in expectations.iter().enumerate() {
            for (alpha, passes) in [64, 127, 128].into_iter().zip(expected) {
                let mut triangle = mesh(0.0, "blue", 1.0);
                triangle["material"]["alpha_flags"] = json!(512 | (mode << 10));
                let frame = render(
                    &scene(vec![triangle]),
                    top(),
                    &textures(&[("blue", [0, 0, 255, alpha])]),
                    &AtomicBool::new(false),
                )
                .unwrap();
                assert_eq!(
                    pixel(&frame, 320, 210),
                    if *passes {
                        [0, 0, 255, 255]
                    } else {
                        BACKGROUND
                    }
                );
            }
        }
        let mut triangle = mesh(0.0, "tiles", 1.0);
        triangle["uvs"] = json!([[-0.25, 1.25], [-0.25, 1.25], [-0.25, 1.25]]);
        let tiles = HashMap::from([(
            "tiles".into(),
            Texture {
                width: 2,
                height: 2,
                rgba: vec![
                    255, 0, 0, 255, 0, 255, 0, 64, 0, 0, 255, 255, 255, 255, 255, 255,
                ],
            },
        )]);
        let frame = render(
            &scene(vec![triangle.clone()]),
            top(),
            &tiles,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(
            pixel(&frame, 320, 210),
            [0, 255, 0, 255],
            "opaque material ignores texture alpha without flags"
        );
        triangle["material"]["alpha_flags"] = json!(4608); // test enabled, greater
        let frame = render(
            &scene(vec![triangle.clone()]),
            top(),
            &tiles,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(pixel(&frame, 320, 210), BACKGROUND);
        triangle["material"]["alpha_threshold"] = json!(42);
        let frame = render(
            &scene(vec![triangle]),
            top(),
            &tiles,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(pixel(&frame, 320, 210), [0, 255, 0, 255]);
        let mut quad = mesh(0.0, "red", 0.5);
        quad["positions"] = json!([[-1, -1, 0], [1, -1, 0], [1, 1, 0], [-1, 1, 0]]);
        quad["normals"] = json!([[0, 0, 1], [0, 0, 1], [0, 0, 1], [0, 0, 1]]);
        quad["uvs"] = json!([]);
        quad["triangles"] = json!([[0, 1, 2], [0, 2, 3]]);
        // Missing UVs intentionally use neutral [160,168,180]. The shared
        // diagonal receives one blend, not two and not a crack.
        let frame = render(
            &scene(vec![quad]),
            top(),
            &HashMap::new(),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(pixel(&frame, 320, 209), [92, 98, 107, 255]);
    }

    #[test]
    fn orbit_zoom_wireframe_backfaces_and_degenerates_are_bounded() {
        let basic = scene(vec![mesh(0.0, "", 1.0)]);
        let maps = HashMap::new();
        let cancel = AtomicBool::new(false);
        let solid = render(&basic, top(), &maps, &cancel).unwrap();
        let count = |f: &Frame| f.rgba.chunks_exact(4).filter(|p| *p != BACKGROUND).count();
        assert_eq!(pixel(&solid, 320, 210), [160, 168, 180, 255]);
        let mut tiny = mesh(0.0, "", 1.0);
        tiny["positions"] = json!([[-1e-15, -1e-15, 0], [1e-15, -1e-15, 0], [0, 1e-15, 0]]);
        assert_eq!(
            render(&scene(vec![tiny]), top(), &maps, &cancel)
                .unwrap()
                .rgba,
            solid.rgba
        );
        let zoom = render(&basic, View { zoom: 2.0, ..top() }, &maps, &cancel).unwrap();
        assert!(count(&zoom) > count(&solid) * 2);
        let orbit = render(
            &basic,
            View {
                yaw: 1.2,
                pitch: 0.45,
                ..top()
            },
            &maps,
            &cancel,
        )
        .unwrap();
        assert_ne!(orbit.rgba, solid.rgba);
        let wire = render(
            &basic,
            View {
                wireframe: true,
                ..top()
            },
            &maps,
            &cancel,
        )
        .unwrap();
        assert_eq!(pixel(&wire, 320, 210), BACKGROUND);
        assert!(count(&wire) > 100 && count(&wire) < count(&solid) / 8);
        let mut reversed = mesh(0.0, "", 1.0);
        reversed["triangles"] = json!([[0, 2, 1]]);
        let back = render(&scene(vec![reversed.clone()]), top(), &maps, &cancel).unwrap();
        assert_eq!(count(&back), 0);
        reversed["material"]["double_sided"] = json!(true);
        let two_sided = render(&scene(vec![reversed]), top(), &maps, &cancel).unwrap();
        assert_eq!(two_sided.rgba, solid.rgba);
        let mut flat = mesh(0.0, "", 1.0);
        flat["positions"] = json!([[0, 0, 0], [0, 0, 0], [0, 0, 0]]);
        assert_eq!(
            count(&render(&scene(vec![flat]), top(), &maps, &cancel).unwrap()),
            0
        );
        assert!(render(
            &basic,
            View {
                zoom: f32::NAN,
                ..top()
            },
            &maps,
            &cancel
        )
        .is_err());
        assert_eq!(
            render(
                &basic,
                View {
                    zoom: 100.0,
                    ..top()
                },
                &maps,
                &cancel
            )
            .unwrap()
            .rgba,
            render(
                &basic,
                View {
                    zoom: 10.0,
                    ..top()
                },
                &maps,
                &cancel
            )
            .unwrap()
            .rgba
        );
    }

    #[test]
    fn malformed_layout_numbers_paths_and_allocation_limits_are_rejected() {
        let input = source(vec![mesh(0.0, "", 1.0)]);
        for (field, value) in [
            ("normals", json!([])),
            ("normals", json!([[0, 0, 0], [0, 0, 1], [0, 0, 1]])),
            ("uvs", json!([[0, 0]])),
            ("positions", json!([[1e10, 0, 0], [0, 0, 0], [1, 1, 0]])),
        ] {
            let mut bad = input.clone();
            bad["meshes"][0][field] = value;
            assert!(parse(&serde_json::to_vec(&bad).unwrap()).is_err());
        }
        let mut bad = input.clone();
        bad["meshes"][0]["material"]["alpha"] = json!(-0.1);
        assert!(parse(&serde_json::to_vec(&bad).unwrap()).is_err());
        let mut bad = input;
        bad["meshes"][0]["material"]["textures"] = json!(["../outside.dds"]);
        let scene = parse(&serde_json::to_vec(&bad).unwrap()).unwrap();
        assert!(scene.meshes[0].diffuse_key.is_none());
        assert!(scene.warnings.iter().any(|s| s.contains("omitted")));
        assert_eq!(
            normalize_texture_path("Textures\\A/./B.DDS").unwrap(),
            "textures/a/b.dds"
        );
        for path in ["/a.dds", "C:\\a.dds", "..\\a.dds", "a\n.dds", ""] {
            assert!(normalize_texture_path(path).is_err());
        }
        assert!(parse(&vec![b' '; MAX_JSON + 1])
            .unwrap_err()
            .contains("64 MiB"));
        let huge = format!(
            "{{\"positions\":[{}]}}",
            vec!["[0,0,0]"; MAX_VERTICES + 1].join(",")
        );
        assert!(serde_json::from_str::<Mesh>(&huge)
            .unwrap_err()
            .to_string()
            .contains("limit"));
        let mut huge = source(vec![mesh(0.0, "", 1.0)]);
        huge["meshes"][0]["triangles"] = json!(vec![[0, 1, 2]; MAX_TRIANGLES + 1]);
        assert!(parse(&serde_json::to_vec(&huge).unwrap()).is_err());
        let too_many = source((0..=MAX_MESHES).map(|_| mesh(0.0, "", 1.0)).collect());
        assert!(parse(&serde_json::to_vec(&too_many).unwrap())
            .unwrap_err()
            .contains("count limit"));
    }

    #[test]
    fn cancellation_texture_limits_and_total_raster_budget_fail_without_partial_frame() {
        let basic = scene(vec![mesh(0.0, "", 1.0)]);
        assert!(
            render(&basic, top(), &HashMap::new(), &AtomicBool::new(true))
                .unwrap_err()
                .contains("cancelled")
        );
        for texture in [
            Texture {
                width: 4097,
                height: 1,
                rgba: vec![],
            },
            Texture {
                width: 1,
                height: 1,
                rgba: vec![0; 3],
            },
        ] {
            assert!(render(
                &basic,
                top(),
                &HashMap::from([("texture".into(), texture)]),
                &AtomicBool::new(false)
            )
            .is_err());
        }
        let large = HashMap::from([
            (
                "a".into(),
                Texture {
                    width: 4096,
                    height: 2049,
                    rgba: vec![0; 4096 * 2049 * 4],
                },
            ),
            (
                "b".into(),
                Texture {
                    width: 4096,
                    height: 2049,
                    rgba: vec![0; 4096 * 2049 * 4],
                },
            ),
        ]);
        assert!(render(&basic, top(), &large, &AtomicBool::new(false))
            .unwrap_err()
            .contains("64 MiB"));
        let too_many = (0..=MAX_MESHES)
            .map(|i| {
                (
                    format!("texture{i}"),
                    Texture {
                        width: 1,
                        height: 1,
                        rgba: vec![0; 4],
                    },
                )
            })
            .collect();
        assert!(render(&basic, top(), &too_many, &AtomicBool::new(false))
            .unwrap_err()
            .contains("texture count"));
        let mut expensive = mesh(0.0, "", 1.0);
        expensive["triangles"] = json!(vec![[0, 1, 2]; 2000]);
        assert!(render(
            &scene(vec![expensive]),
            top(),
            &HashMap::new(),
            &AtomicBool::new(false)
        )
        .unwrap_err()
        .contains("pixel work limit"));
        let empty = parse(&serde_json::to_vec(&source(vec![])).unwrap()).unwrap();
        assert!(
            render(&empty, top(), &HashMap::new(), &AtomicBool::new(false))
                .unwrap_err()
                .contains("No static")
        );
    }
}
