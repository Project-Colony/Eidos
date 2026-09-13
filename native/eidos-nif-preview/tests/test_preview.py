"""Golden world-space geometry and malformed-input acceptance checks."""
import json
import math
from pathlib import Path
import subprocess
import struct
import sys
import tempfile

helper, fixture_generator = sys.argv[1:]
executions = 0

def layout(data):
    """Locate synthetic header tables for independent byte-level corruption."""
    pos = data.index(b"\n") + 10
    count, stream = struct.unpack_from("<II", data, pos)
    pos += 8
    for _ in range(3):
        pos += 1 + data[pos]
    types, = struct.unpack_from("<H", data, pos)
    pos += 2
    names = []
    for _ in range(types):
        size, = struct.unpack_from("<I", data, pos)
        pos += 4
        names.append(data[pos:pos+size].decode())
        pos += size
    indices = struct.unpack_from(f"<{count}H", data, pos)
    pos += count * 2
    size_table = pos
    sizes = struct.unpack_from(f"<{count}I", data, pos)
    pos += count * 4
    strings, = struct.unpack_from("<I", data, pos)
    pos += 8
    for _ in range(strings):
        size, = struct.unpack_from("<I", data, pos)
        pos += 4 + size
    groups, = struct.unpack_from("<I", data, pos)
    pos += 4 + groups * 4
    blocks = {}
    for index, (kind, size) in enumerate(zip(indices, sizes)):
        blocks.setdefault(names[kind], []).append((index, pos, size))
        pos += size
    return blocks, size_table, pos

def run(path, error=None):
    global executions
    executions += 1
    result = subprocess.run([helper, str(path)], capture_output=True, timeout=15)
    if error is not None:
        assert result.returncode != 0, (path, result.stdout[:200])
        assert not result.stdout, (path, "partial JSON on failure")
        assert error in result.stderr.decode(), (path, result.stderr)
        return
    assert result.returncode == 0, (path, result.stderr)
    assert len(result.stdout) <= 64 * 1024 * 1024
    return json.loads(result.stdout)

def near(actual, expected, tolerance=0.01):
    assert len(actual) == len(expected), (actual, expected)
    for a, e in zip(actual, expected):
        if isinstance(e, list):
            near(a, e, tolerance)
        else:
            assert math.isclose(a, e, abs_tol=tolerance), (actual, expected)

with tempfile.TemporaryDirectory(prefix="eidos-nif-tests-") as directory:
    directory = Path(directory)
    subprocess.run([fixture_generator, str(directory)], check=True, timeout=30)
    for game, stream, block_type in [("le", 83, "NiTriShape"), ("sse", 100, "BSTriShape")]:
        data = run(directory / f"{game}-valid.nif")
        assert data["version"] == 1 and data["stream_version"] == stream
        assert data["nif_version"] == "20.2.0.7" and data["user_version"] == 12
        assert not data["warnings"], data["warnings"]
        mesh, = data["meshes"]
        assert mesh["block_type"] == block_type and mesh["name"] == 'Synthetic "triangle"'
        near(mesh["positions"], [[10,22,30], [10,24,30], [8,22,30]])
        near(mesh["normals"], [[0,0,1]] * 3)
        near(mesh["uvs"], [[0,0], [1,0], [0,1]])
        assert mesh["triangles"] == [[0,1,2]]
        assert data["bounds"] == {"min":[8,22,30], "max":[10,24,30]}
        material = mesh["material"]
        assert material["textures"][:2] == ["textures\\synthetic\\diffuse.dds", "textures\\synthetic\\normal.dds"]
        assert material["double_sided"] and material["alpha"] == 0.5
        assert material["alpha_flags"] == 4845 and material["alpha_threshold"] == 42
        mirrored = run(directory / f"{game}-mirrored.nif")["meshes"][0]
        near(mirrored["positions"], [[10,22,30], [10,20,30], [12,22,30]])
        near(mirrored["normals"], [[0,0,-1]] * 3)
        assert mirrored["triangles"] == [[0,2,1]]
        shear = run(directory / f"{game}-shear.nif")["meshes"][0]
        near(shear["normals"], [[1/math.sqrt(5),2/math.sqrt(5),0]] * 3)
        uv_transform = run(directory / f"{game}-uv-transform.nif")["meshes"][0]
        near(uv_transform["uvs"], [[0.25,0.5], [2.25,0.5], [0.25,3.5]])
        no_normals = run(directory / f"{game}-no-normals.nif")
        near(no_normals["meshes"][0]["normals"], [[0,0,1]] * 3)
        assert any("normals" in w for w in no_normals["warnings"])
        unsupported = run(directory / f"{game}-unsupported.nif")
        assert len(unsupported["meshes"]) == 1
        assert any("NiIntegerExtraData" in w for w in unsupported["warnings"])
        skinned = run(directory / f"{game}-skinned.nif")
        assert not skinned["meshes"] and skinned["bounds"] is None
        assert any("skinned" in w for w in skinned["warnings"])
        for mode, reason in [("bad-index","triangle"), ("nan","finite"), ("singular","singular"),
                             ("cycle","cycle"), ("dangling","reference"), ("deep","depth"),
                             ("many-vertices","vertex limit"), ("bad-alpha","alpha"),
                             ("duplicate-parent","multiple parents"), ("bad-utf8","UTF-8"),
                             ("output-limit","JSON output limit")]:
            run(directory / f"{game}-{mode}.nif", reason)
        original = (directory / f"{game}-valid.nif").read_bytes()
        blocks, size_table, footer = layout(original)
        shape_index, shape_offset, shape_size = blocks[block_type][0]
        mutations = [(shape_offset, 999999, "string reference"), (footer + 4, 999999, "reference"),
                     (size_table + shape_index * 4, shape_size + 1, "block size")]
        if game == "sse":
            stored_size, = struct.unpack_from("<I", original, shape_offset + 112)
            mutations += [(shape_offset + 112, stored_size + 1, "data size")]
            descriptor_low, = struct.unpack_from("<I", original, shape_offset + 100)
            mutations += [(shape_offset + 100, descriptor_low ^ 1, "vertex description")]
        for offset, value, reason in mutations:
            corrupt = bytearray(original)
            struct.pack_into("<I", corrupt, offset, value)
            path = directory / "mutated.nif"
            path.write_bytes(corrupt)
            run(path, reason)
        for length in [0, 10, 39, 80, len(original)//2, len(original)-1]:
            path = directory / "truncated.nif"
            path.write_bytes(original[:length])
            run(path, "NIF")
        path = directory / "trailing.nif"
        path.write_bytes(original + b"\x00")
        run(path, "trailing")
        path = directory / "endian.nif"
        corrupt = bytearray(original)
        corrupt[original.index(b"\n") + 5] = 0
        path.write_bytes(corrupt)
        run(path, "endian")
    large = directory / "too-large.nif"
    with large.open("wb") as stream:
        stream.truncate(64 * 1024 * 1024 + 1)
    run(large, "input limit")
    run(directory, "regular file")
    fifo = directory / "pipe.nif"
    import os
    os.mkfifo(fifo)
    run(fifo, "regular file")
    link = directory / "link.nif"
    link.symlink_to(directory / "le-valid.nif")
    run(link, "regular file")
    for game in ["le", "sse"]:
        checked_in = Path(__file__).parent / "fixtures" / f"{game}-valid.nif"
        assert checked_in.read_bytes() == (directory / f"{game}-valid.nif").read_bytes()
        run(checked_in)
print(f"{executions} NIF subprocess checks passed: golden geometry, transforms, materials, malformed inputs and limits")
