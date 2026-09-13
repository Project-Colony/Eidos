// Eidos static NIF preview helper. See UPSTREAM.md for parser
// provenance/licenses.
#include "Factory.hpp"
#include "Geometry.hpp"
#include "Nodes.hpp"
#include "Shaders.hpp"
#include <cerrno>
#include <cmath>
#include <fcntl.h>
#include <functional>
#include <iomanip>
#include <iostream>
#include <locale>
#include <sstream>
#include <stdexcept>
#include <string_view>
#include <sys/resource.h>
#include <sys/stat.h>
#include <unistd.h>

using namespace nifly;

namespace {
constexpr size_t NIF_INPUT_LIMIT = 64 * 1024 * 1024;
constexpr size_t MAX_OUTPUT = 64 * 1024 * 1024;
constexpr size_t MAX_VERTICES = 250000;
constexpr size_t MAX_TRIANGLES = 500000;
constexpr size_t MAX_MESHES = 1024;
constexpr uint32_t MAX_BLOCKS = 20000;
constexpr unsigned MAX_DEPTH = 256;

void require(bool condition, const std::string &reason) {
  if (!condition)
    throw std::runtime_error(reason);
}

void limit(int resource, rlim_t maximum) {
  rlimit current{};
  require(getrlimit(resource, &current) == 0, "cannot read process limits");
  current.rlim_cur = std::min(current.rlim_cur, maximum);
  require(setrlimit(resource, &current) == 0, "cannot set process limits");
}

struct Descriptor {
  int fd;
  ~Descriptor() {
    if (fd >= 0)
      close(fd);
  }
};

std::string read_input(const char *path) {
  Descriptor input{open(path, O_RDONLY | O_CLOEXEC | O_NOFOLLOW | O_NONBLOCK)};
  require(input.fd >= 0, "cannot open input as a regular file");
  struct stat before{}, after{};
  require(fstat(input.fd, &before) == 0 && S_ISREG(before.st_mode),
          "input must be a regular file");
  require(before.st_size >= 0 && uint64_t(before.st_size) <= NIF_INPUT_LIMIT,
          "input limit is 64 MiB");
  std::string bytes(static_cast<size_t>(before.st_size), '\0');
  size_t offset = 0;
  while (offset < bytes.size()) {
    auto count = read(input.fd, bytes.data() + offset, bytes.size() - offset);
    if (count < 0 && errno == EINTR)
      continue;
    require(count > 0, "NIF input changed or could not be read");
    offset += static_cast<size_t>(count);
  }
  char extra;
  require(read(input.fd, &extra, 1) == 0 && fstat(input.fd, &after) == 0,
          "NIF input changed during read");
  require(before.st_size == after.st_size &&
              before.st_mtim.tv_sec == after.st_mtim.tv_sec &&
              before.st_mtim.tv_nsec == after.st_mtim.tv_nsec &&
              before.st_ctim.tv_sec == after.st_ctim.tv_sec &&
              before.st_ctim.tv_nsec == after.st_ctim.tv_nsec,
          "NIF input changed during read");
  return bytes;
}

// Limit each upstream block parser to its declared bytes and throw on short
// reads.
class ReadBuffer : public std::streambuf {
public:
  explicit ReadBuffer(std::string_view bytes) {
    auto begin = const_cast<char *>(bytes.data());
    setg(begin, begin, begin + bytes.size());
  }
  size_t remaining() const { return static_cast<size_t>(egptr() - gptr()); }
};

class JsonBuffer : public std::streambuf {
public:
  std::string bytes;
  std::streamsize xsputn(const char *data, std::streamsize count) override {
    require(count >= 0 &&
                static_cast<size_t>(count) <= MAX_OUTPUT - bytes.size(),
            "JSON output limit is 64 MiB");
    bytes.append(data, static_cast<size_t>(count));
    return count;
  }
  int_type overflow(int_type c) override {
    if (!traits_type::eq_int_type(c, traits_type::eof())) {
      char byte = traits_type::to_char_type(c);
      xsputn(&byte, 1);
    }
    return traits_type::not_eof(c);
  }
};

void string_json(std::ostream &out, const std::string &value) {
  require(value.size() <= 4096, "NIF string limit is 4096 bytes");
  // Validate UTF-8 instead of emitting malformed JSON or changing texture
  // paths.
  for (size_t i = 0; i < value.size();) {
    const auto first = static_cast<unsigned char>(value[i++]);
    if (first < 128)
      continue;
    unsigned count = first >= 0xf0 && first <= 0xf4   ? 3
                     : first >= 0xe0 && first <= 0xef ? 2
                     : first >= 0xc2 && first <= 0xdf ? 1
                                                      : 0;
    require(count > 0 && count <= value.size() - i,
            "NIF string is not valid UTF-8");
    unsigned code = first & ((1u << (6 - count)) - 1);
    for (unsigned j = 0; j < count; ++j) {
      const auto next = static_cast<unsigned char>(value[i++]);
      require((next & 0xc0) == 0x80, "NIF string is not valid UTF-8");
      code = (code << 6) | (next & 0x3f);
    }
    require(code >= (count == 1   ? 0x80u
                     : count == 2 ? 0x800u
                                  : 0x10000u) &&
                code <= 0x10ffff && !(code >= 0xd800 && code <= 0xdfff),
            "NIF string is not valid UTF-8");
  }
  out << '"';
  constexpr char hex[] = "0123456789abcdef";
  for (unsigned char c : value) {
    if (c == '"' || c == '\\')
      out << '\\' << char(c);
    else if (c < 0x20)
      out << "\\u00" << hex[c >> 4] << hex[c & 15];
    else
      out << char(c);
  }
  out << '"';
}

void finite(float value) {
  require(std::isfinite(value), "geometry and materials must be finite");
  require(std::abs(value) <= 1e9f, "geometry component limit is 1e9");
}
void finite(const Vector3 &v) {
  finite(v.x);
  finite(v.y);
  finite(v.z);
}
void vector_json(std::ostream &out, const Vector3 &v) {
  finite(v);
  out << '[' << v.x << ',' << v.y << ',' << v.z << ']';
}

Vector3 normalized(Vector3 normal) {
  finite(normal);
  double length =
      std::hypot(double(normal.x), double(normal.y), double(normal.z));
  require(length > 1e-20, "zero or degenerate normal");
  return Vector3(float(normal.x / length), float(normal.y / length),
                 float(normal.z / length));
}

Matrix3 normal_matrix(const MatTransform &transform) {
  finite(transform.translation);
  finite(transform.scale);
  for (int i = 0; i < 3; ++i)
    finite(transform.rotation[i]);
  Matrix3 inverse;
  require(std::abs(transform.scale) >= 1e-12f &&
              transform.rotation.Invert(&inverse),
          "singular transform is unsupported");
  auto result = inverse.Transpose();
  for (int i = 0; i < 3; ++i)
    result[i] *= 1.0f / transform.scale;
  return result;
}

std::string vertex_description(VertexDesc description, NiHeader &header) {
  std::ostringstream bytes;
  NiOStream output(&bytes, &header);
  NiStreamReversible stream(nullptr, &output,
                            NiStreamReversible::Mode::Writing);
  description.Sync(stream);
  return bytes.str();
}

struct Scene {
  NiHeader header;
  std::vector<std::unique_ptr<NiObject>> blocks;
  std::vector<std::string> types;
  std::vector<std::string> warnings;

  void warning(uint32_t id, const std::string &reason) {
    warnings.push_back("Block " + std::to_string(id) + " (" + types[id] +
                       "): " + reason);
  }

  void parse(const std::string &bytes) {
    const auto newline = bytes.find('\n');
    require(newline != std::string::npos && newline < 128 &&
                bytes.size() >= newline + 6,
            "NIF header is truncated");
    require(std::string_view(bytes).substr(newline + 1, 4) ==
                std::string_view("\x07\x00\x02\x14", 4),
            "unsupported NIF version; expected Skyrim LE/SSE 20.2.0.7");
    require(bytes[newline + 5] == 1,
            "unsupported NIF endian; expected little endian");
    ReadBuffer headerBuffer(bytes);
    std::istream input(&headerBuffer);
    input.exceptions(std::ios::failbit | std::ios::badbit);
    NiIStream stream(&input, &header);
    header.Get(stream);
    auto version = header.GetVersion();
    require(header.IsValid() && version.User() == 12 &&
                (version.IsSK() || version.IsSSE()),
            "unsupported or malformed NIF header; expected Skyrim LE/SSE");
    auto count = header.GetNumBlocks();
    require(count > 0 && count <= MAX_BLOCKS, "NIF block limit is 20000");
    require(header.GetStringCount() <= MAX_BLOCKS,
            "NIF string count limit is 20000");
    for (uint32_t i = 0; i < header.GetStringCount(); ++i)
      require(header.GetStringById(i).size() <= 4096,
              "NIF string limit is 4096 bytes");
    blocks.resize(count);
    types.resize(count);
    header.SetBlockReference(&blocks);
    size_t offset = bytes.size() - headerBuffer.remaining();
    const std::set<std::string> supported{"NiNode",
                                          "BSFadeNode",
                                          "NiTriShape",
                                          "NiTriShapeData",
                                          "BSTriShape",
                                          "BSLightingShaderProperty",
                                          "BSShaderTextureSet",
                                          "NiAlphaProperty"};
    for (uint32_t i = 0; i < count; ++i) {
      types[i] = header.GetBlockTypeStringById(i);
      require(types[i].size() <= 128, "NIF block type name limit is 128 bytes");
      const auto size = header.GetBlockSize(i);
      require(size <= bytes.size() - offset, "NIF block is truncated");
      if (supported.count(types[i])) {
        ReadBuffer buffer(std::string_view(bytes).substr(offset, size));
        std::istream blockInput(&buffer);
        blockInput.exceptions(std::ios::failbit | std::ios::badbit);
        NiIStream blockStream(&blockInput, &header);
        auto factory = NiFactoryRegister::Get().GetFactoryByName(types[i]);
        require(factory != nullptr, "NIF parser factory is missing");
        blocks[i] = factory->Load(blockStream);
        require(buffer.remaining() == 0,
                "NIF block size does not match parsed data");
      } else {
        blocks[i] = std::make_unique<NiUnknown>();
        warning(i, "unsupported block skipped (animation, skinning, collision "
                   "and other effects are not evaluated)");
      }
      offset += size;
    }
    ReadBuffer footerBuffer(std::string_view(bytes).substr(offset));
    std::istream footerInput(&footerBuffer);
    footerInput.exceptions(std::ios::failbit | std::ios::badbit);
    NiIStream footerStream(&footerInput, &header);
    header.GetFooter(footerStream);
    require(footerBuffer.remaining() == 0, "NIF has trailing data");
    require(!header.GetRootBlockIds().empty() &&
                header.GetRootBlockIds().size() <= count,
            "NIF root count is invalid");
    for (uint32_t i = 0; i < count; ++i) {
      std::vector<NiStringRef *> strings;
      blocks[i]->GetStringRefs(strings);
      for (auto ref : strings) {
        require(ref->GetIndex() == NIF_NPOS ||
                    ref->GetIndex() < header.GetStringCount(),
                "NIF string reference is out of bounds");
        ref->get() = header.GetStringById(ref->GetIndex());
      }
      std::set<NiRef *> refs;
      blocks[i]->GetChildRefs(refs);
      for (auto ref : refs)
        require(ref->IsEmpty() || ref->index < count,
                "NIF block reference is out of bounds");
      if (auto shape = dynamic_cast<NiTriShape *>(blocks[i].get())) {
        auto data = header.GetBlock<NiTriShapeData>(shape->DataRef());
        require(data != nullptr,
                "NiTriShape geometry reference must name NiTriShapeData");
        shape->SetGeomData(data);
      }
    }
  }

  NiShader *shader_for(NiShape *shape) {
    auto ref = shape->ShaderPropertyRef();
    if (!ref || ref->IsEmpty())
      return nullptr;
    auto shader = header.GetBlock<NiShader>(ref);
    require(shader || dynamic_cast<NiUnknown *>(blocks[ref->index].get()),
            "NIF shader reference has the wrong type");
    return shader;
  }

  void validate_graph() {
    std::vector<unsigned char> state(blocks.size());
    std::function<void(uint32_t, unsigned)> visit = [&](uint32_t id,
                                                        unsigned depth) {
      require(id < blocks.size(), "NIF scene reference is out of bounds");
      require(depth <= MAX_DEPTH, "NIF scene depth limit is 256");
      require(state[id] != 1, "NIF scene graph contains a cycle");
      if (state[id] == 2)
        return;
      state[id] = 1;
      if (auto node = dynamic_cast<NiNode *>(blocks[id].get())) {
        for (auto child : node->childRefs) {
          if (child.IsEmpty())
            continue;
          require(child.index < blocks.size(),
                  "NIF scene reference is out of bounds");
          require(dynamic_cast<NiAVObject *>(blocks[child.index].get()) ||
                      dynamic_cast<NiUnknown *>(blocks[child.index].get()),
                  "NIF child reference is not a scene object");
          visit(child.index, depth + 1);
        }
      }
      state[id] = 2;
    };
    for (uint32_t i = 0; i < blocks.size(); ++i)
      visit(i, 0);
  }

  std::string json() {
    validate_graph();
    JsonBuffer buffer;
    std::ostream out(&buffer);
    out.exceptions(std::ios::failbit | std::ios::badbit);
    out.imbue(std::locale::classic());
    out << std::setprecision(9)
        << "{\"version\":1,\"nif_version\":\"20.2.0.7\",\"user_version\":12,"
           "\"stream_version\":"
        << header.GetVersion().Stream() << ",\"meshes\":[";
    size_t vertices = 0, triangles = 0, meshes = 0;
    Vector3 minimum, maximum;
    std::vector<bool> seen(blocks.size());
    std::function<void(uint32_t, const MatTransform &, unsigned)> visit;
    visit = [&](uint32_t id, const MatTransform &parent, unsigned depth) {
      require(id < blocks.size(), "NIF root reference is out of bounds");
      require(depth <= MAX_DEPTH, "NIF scene depth limit is 256");
      require(!seen[id],
              "NIF scene object has multiple parents or duplicate roots");
      seen[id] = true;
      auto object = dynamic_cast<NiAVObject *>(blocks[id].get());
      if (!object) {
        warning(id, "scene object cannot be rendered");
        return;
      }
      normal_matrix(object->transform);
      auto world = parent.ComposeTransforms(object->transform);
      auto normalTransform = normal_matrix(world);
      if (auto node = dynamic_cast<NiNode *>(object)) {
        for (auto child : node->childRefs)
          if (!child.IsEmpty())
            visit(child.index, world, depth + 1);
        return;
      }
      auto shape = dynamic_cast<NiShape *>(object);
      if (!shape) {
        warning(id, "unsupported scene object");
        return;
      }
      auto shader = shader_for(shape);
      if (shape->IsSkinned() || shape->HasSkinInstance() ||
          (shader && shader->IsSkinned())) {
        warning(id, "skinned geometry is unsupported and was skipped");
        return;
      }
      std::vector<Vector3> positions, normals;
      std::vector<Vector2> uvs;
      std::vector<Triangle> indices;
      require(shape->HasVertices(), "NIF shape has no vertex data");
      require(vertices + shape->GetNumVertices() <= MAX_VERTICES,
              "NIF vertex limit is 250000");
      require(triangles + shape->GetNumTriangles() <= MAX_TRIANGLES,
              "NIF triangle limit is 500000");
      if (auto data = shape->GetGeomData()) {
        positions = data->vertices;
        if (shape->HasNormals())
          normals = data->normals;
        if (!data->uvSets.empty())
          uvs = data->uvSets[0];
        if (data->uvSets.size() > 1)
          warning(id, "only the first UV set is displayed");
      } else if (auto bs = dynamic_cast<BSTriShape *>(shape)) {
        require(!bs->HasSecondUVs(),
                "BSTriShape second UV layout is unsupported");
        const uint32_t storedSize = bs->dataSize;
        const auto storedDescription =
            vertex_description(bs->vertexDesc, header);
        require(bs->CalcDataSizes(header.GetVersion()) == int(storedSize),
                "BSTriShape declared data size is inconsistent");
        require(
            storedDescription == vertex_description(bs->vertexDesc, header),
            "BSTriShape vertex description has inconsistent offsets or stride");
        positions = bs->UpdateRawVertices();
        if (shape->HasNormals())
          normals = bs->UpdateRawNormals();
        if (shape->HasUVs())
          uvs = bs->UpdateRawUvs();
        if (bs->particleDataSize)
          warning(id, "particle geometry is not displayed");
      }
      require(positions.size() == shape->GetNumVertices(),
              "NIF vertex count is inconsistent");
      require(shape->GetTriangles(indices) &&
                  indices.size() == shape->GetNumTriangles(),
              "NIF triangle count is inconsistent");
      require(normals.empty() || normals.size() == positions.size(),
              "NIF normal count is inconsistent");
      require(uvs.empty() || uvs.size() == positions.size(),
              "NIF UV count is inconsistent");
      for (const auto &p : positions)
        finite(p);
      for (const auto &t : indices)
        require(t.p1 < positions.size() && t.p2 < positions.size() &&
                    t.p3 < positions.size(),
                "NIF triangle index is out of bounds");
      if (indices.empty()) {
        warning(id, "empty geometry was skipped");
        return;
      }
      if (normals.empty()) {
        warning(id, "missing normals were generated from triangle areas");
        normals.resize(positions.size());
        for (const auto &t : indices) {
          auto n = (positions[t.p2] - positions[t.p1])
                       .cross(positions[t.p3] - positions[t.p1]);
          normals[t.p1] += n;
          normals[t.p2] += n;
          normals[t.p3] += n;
        }
        bool fallback = false;
        for (auto &n : normals)
          if (n.IsZero()) {
            n = Vector3(0, 0, 1);
            fallback = true;
          }
        if (fallback)
          warning(id, "unused or degenerate vertices use a +Z fallback normal");
      }
      bool mirrored = (world.rotation.Determinant() < 0) != (world.scale < 0);
      if (mirrored)
        for (auto &t : indices)
          std::swap(t.p2, t.p3);
      for (size_t i = 0; i < positions.size(); ++i) {
        positions[i] = world.ApplyTransform(positions[i]);
        finite(positions[i]);
        normals[i] = normalized(normalTransform * normals[i]);
        if (!vertices && !i)
          minimum = maximum = positions[i];
        for (int axis = 0; axis < 3; ++axis) {
          minimum[axis] = std::min(minimum[axis], positions[i][axis]);
          maximum[axis] = std::max(maximum[axis], positions[i][axis]);
        }
      }
      auto uvScale = shader ? shader->GetUVScale() : Vector2(1, 1);
      auto uvOffset = shader ? shader->GetUVOffset() : Vector2();
      for (auto &uv : uvs) {
        finite(uv.u);
        finite(uv.v);
        uv.u = uv.u * uvScale.u + uvOffset.u;
        uv.v = uv.v * uvScale.v + uvOffset.v;
        finite(uv.u);
        finite(uv.v);
      }
      if (shape->HasVertexColors())
        warning(id, "vertex colors are not displayed");
      require(meshes < MAX_MESHES, "NIF mesh limit is 1024");
      if (meshes++)
        out << ',';
      vertices += positions.size();
      triangles += indices.size();
      out << "{\"block\":" << id << ",\"name\":";
      string_json(out, object->name.get());
      out << ",\"block_type\":";
      string_json(out, types[id]);
      out << ",\"positions\":[";
      for (size_t i = 0; i < positions.size(); ++i) {
        if (i)
          out << ',';
        vector_json(out, positions[i]);
      }
      out << "],\"normals\":[";
      for (size_t i = 0; i < normals.size(); ++i) {
        if (i)
          out << ',';
        vector_json(out, normals[i]);
      }
      out << "],\"uvs\":[";
      for (size_t i = 0; i < uvs.size(); ++i) {
        if (i)
          out << ',';
        out << '[' << uvs[i].u << ',' << uvs[i].v << ']';
      }
      out << "],\"triangles\":[";
      for (size_t i = 0; i < indices.size(); ++i) {
        if (i)
          out << ',';
        const auto &t = indices[i];
        out << '[' << t.p1 << ',' << t.p2 << ',' << t.p3 << ']';
      }
      out << "],\"material\":{\"shader\":";
      string_json(out, shader ? shader->GetBlockName() : "");
      out << ",\"textures\":[";
      if (shader && shader->HasTextureSet()) {
        auto textures = header.GetBlock(shader->TextureSetRef());
        require(textures != nullptr,
                "NIF texture set reference has the wrong type");
        require(textures->textures.size() <= 16,
                "NIF texture slot limit is 16");
        for (size_t i = 0; i < textures->textures.size(); ++i) {
          if (i)
            out << ',';
          string_json(out, textures->textures[i].get());
        }
      }
      float alpha = shader ? shader->GetAlpha() : 1.0f;
      finite(alpha);
      require(alpha >= 0 && alpha <= 1, "NIF material alpha is outside 0..1");
      auto alphaProperty =
          header.GetBlock<NiAlphaProperty>(shape->AlphaPropertyRef());
      require(!shape->HasAlphaProperty() || alphaProperty,
              "NIF alpha property reference has the wrong type");
      out << "],\"alpha\":" << alpha << ",\"double_sided\":"
          << (shader && shader->IsDoubleSided() ? "true" : "false")
          << ",\"alpha_flags\":" << (alphaProperty ? alphaProperty->flags : 0)
          << ",\"alpha_threshold\":"
          << (alphaProperty ? unsigned(alphaProperty->threshold) : 0) << "}}";
    };
    MatTransform identity;
    for (auto root : header.GetRootBlockIds())
      visit(root, identity, 0);
    for (uint32_t i = 0; i < blocks.size(); ++i)
      if (!seen[i] && dynamic_cast<NiShape *>(blocks[i].get()))
        warning(i, "shape is outside the root scene and was skipped");
    out << "],\"bounds\":";
    if (meshes) {
      out << "{\"min\":";
      vector_json(out, minimum);
      out << ",\"max\":";
      vector_json(out, maximum);
      out << '}';
    } else
      out << "null";
    out << ",\"warnings\":[";
    for (size_t i = 0; i < warnings.size(); ++i) {
      if (i)
        out << ',';
      string_json(out, warnings[i]);
    }
    out << "]}\n";
    return std::move(buffer.bytes);
  }
};
} // namespace

int main(int argc, char **argv) {
  try {
    require(argc == 2, "usage: eidos-nif-preview input.nif");
    limit(RLIMIT_AS, 512 * 1024 * 1024);
    limit(RLIMIT_CPU, 10);
    limit(RLIMIT_CORE, 0);
    auto bytes = read_input(argv[1]);
    Scene scene;
    scene.parse(bytes);
    auto json = scene.json();
    std::cout.write(json.data(), static_cast<std::streamsize>(json.size()));
    return std::cout ? 0 : 1;
  } catch (const std::bad_alloc &) {
    std::cerr << "NIF preview: memory limit exceeded\n";
  } catch (const std::exception &error) {
    std::cerr << "NIF preview: " << error.what() << '\n';
  }
  return 1;
}
