# Static NIF preview helper

Build with a C++17 compiler and CMake 3.20 or newer on Linux. Python 3 is needed only for tests. No system installation, asset downloads, Rust dependency or graphics library is required.

```sh
cmake -S native/eidos-nif-preview -B /tmp/eidos-nif-build -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=ON
cmake --build /tmp/eidos-nif-build --parallel 2
ctest --test-dir /tmp/eidos-nif-build --output-on-failure
/tmp/eidos-nif-build/eidos-nif-preview input.nif
```

For the GUI, keep `eidos-nif-preview` beside `eidos-gui` or add its absolute
directory to `PATH`. `just build` and release packaging do this automatically.
For direct GUI tests, use
`PATH=/tmp/eidos-nif-build:$PATH cargo test -p eidos-gui -- --test-threads=1`.

The GUI provides orbit, tilt, zoom and wireframe controls. It loads diffuse
textures from the current loose/archive winners after archive analysis completes;
missing, changed or uncertain providers are reported in the preview. Reopen the
model after refreshing providers. Rendering is a 640×420 orthographic static
view with approximate lighting, alpha testing and basic transparency, not an
engine shader or animation preview. Normal maps and material effects are not
evaluated. At most 64 diffuse textures are attempted, within 256 MiB of input
and 64 MiB of decoded pixels. A texture larger than 4096 pixels uses an existing
smaller mip or is reported unsupported. Cube/array diffuse textures are unsupported.

`nif-preview-fixtures output-directory` creates synthetic LE/SSE files for tests and GUI smoke checks. They contain a triangle and synthetic texture references, with no redistributed game content. Tests assert fixed world-space coordinates, rather than merely round-tripping parser output.

## Protocol version 1

The sole argument is an input file path. It must be a regular file, not a symlink. The helper reads it once, checks size/change metadata, opens no referenced textures, and writes one UTF-8 JSON object to stdout only after complete success. Exit 0 indicates complete output; nonzero indicates failure with a short reason on stderr. The caller must reject nonzero exits, signals, invalid JSON and missing output. CPU-limit termination may have no stderr. Apply a 15-second wall timeout and capture stdout up to 64 MiB plus a byte to detect overflow. The input should be a private temporary file populated from the already validated archive/loose byte source.

```json
{
  "version": 1,
  "nif_version": "20.2.0.7",
  "user_version": 12,
  "stream_version": 83,
  "bounds": {"min": [8,22,30], "max": [10,24,30]},
  "meshes": [{
    "block": 4,
    "name": "Synthetic triangle",
    "block_type": "NiTriShape",
    "positions": [[10,22,30], [10,24,30], [8,22,30]],
    "normals": [[0,0,1], [0,0,1], [0,0,1]],
    "uvs": [[0,0], [1,0], [0,1]],
    "triangles": [[0,1,2]],
    "material": {
      "shader": "BSLightingShaderProperty",
      "textures": ["textures\\synthetic\\diffuse.dds", "textures\\synthetic\\normal.dds"],
      "alpha": 0.5,
      "double_sided": true,
      "alpha_flags": 4845,
      "alpha_threshold": 42
    }
  }],
  "warnings": []
}
```

Stream 83 is Skyrim LE; 100 is SSE. Meshes retain file/scene traversal order. Triangle indices are zero-based within each mesh. Positions and normals are in NIF world coordinates, with parent transforms composed in order. Normals use inverse transpose and normalization; mirrored transforms swap the second and third triangle indices. Missing normals are area-weighted from triangles, with a visible warning; unused or degenerate vertices fall back to +Z with another warning. UVs retain NIF orientation with the lighting shader's UV scale/offset applied. Missing UVs produce `[]`. Texture strings retain their original slot positions and spelling; diffuse is slot 0, normal slot 1. No texture is resolved by the helper.

Material alpha, double-sided state, and raw `NiAlphaProperty` flags/threshold are preserved for the renderer. Empty shader/texture slots remain empty. Missing textures are a caller diagnostic. Bounds span the returned world-space positions; `bounds` is `null` when no mesh can be returned. An empty `meshes` array must be displayed as unavailable static geometry, with the warnings retained.

## Supported scope and limits

Supported version: little-endian 20.2.0.7, user version 12, stream 83/100. Supported blocks: `NiNode`, `BSFadeNode`, `NiTriShape`, `NiTriShapeData`, `BSTriShape`, `BSLightingShaderProperty`, `BSShaderTextureSet`, `NiAlphaProperty`. Other blocks are explicitly reported by block index/type and skipped. Shapes outside the root scene are reported and skipped. Skinned shapes are skipped, including shader-marked skinning. Animation, bone posing, collision, particles, LOD/switch node semantics, vertex colors, other shader families and full engine shader effects are not evaluated. This helper is a static geometry backend; GUI interaction/rendering is separate.

| Limit | Maximum |
|---|---:|
| Input bytes | 64 MiB |
| Complete stdout JSON | 64 MiB |
| Process address space | 512 MiB |
| CPU time | 10 seconds |
| Blocks / header strings | 20,000 each |
| String / block-type bytes | 4,096 / 128 |
| Root scene depth | 256 edges |
| Returned meshes | 1,024 |
| Returned vertices / triangles | 250,000 / 500,000 |
| Texture slots per material | 16 |
| Absolute position/UV/transform component | 1e9 |

Malformed references, cycles, duplicate parents/roots, incomplete or extra block bytes, invalid indices, inconsistent vertex layouts/sizes, nonfinite geometry and singular transforms are errors. Limits are errors with no partial successful mesh output. Legacy non-UTF-8 names/texture paths are rejected explicitly. Second-UV SSE layouts are unsupported. The isolated helper and process limits contain parser resource use; they are not a general filesystem/network security sandbox. No files are written by the helper.
