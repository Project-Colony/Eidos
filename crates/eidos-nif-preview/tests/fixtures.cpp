// Synthetic geometry only; no game files or assets are included.
#include "NifFile.hpp"
#include <filesystem>
#include <limits>
#include <stdexcept>

using namespace nifly;

int main(int argc, char **argv) {
  if (argc != 2)
    return 1;
  std::filesystem::create_directories(argv[1]);
  for (bool sse : {false, true}) {
    for (const std::string mode :
         {"valid", "mirrored", "bad-index", "nan", "cycle", "dangling",
          "skinned", "unsupported", "no-normals", "singular", "shear", "deep",
          "many-vertices", "output-limit", "bad-alpha", "uv-transform",
          "duplicate-parent", "bad-utf8"}) {
      NifFile nif;
      nif.Create(sse ? NiVersion::getSSE() : NiVersion::getSK());
      auto &hdr = nif.GetHeader();
      std::vector<Vector3> vertices{{0, 0, 0}, {1, 0, 0}, {0, 1, 0}};
      std::vector<Vector3> normals(3, Vector3(0, 0, 1));
      std::vector<Vector2> uvs{{0, 0}, {1, 0}, {0, 1}};
      std::vector<Triangle> triangles{{0, 1, 2}};
      if (mode == "shear")
        normals.assign(3, Vector3(0.70710678f, 0.70710678f, 0));
      if (mode == "many-vertices") {
        vertices.resize(60000);
        normals.resize(60000, Vector3(0, 0, 1));
        uvs.resize(60000);
      }
      auto shape = nif.CreateShapeFromData(
          "Synthetic \"triangle\"", &vertices, &triangles, &uvs,
          mode == "no-normals" ? nullptr : &normals);
      auto shader = nif.GetShader(shape);
      auto textureSet = hdr.GetBlock(shader->TextureSetRef());
      textureSet->textures[0].get() = "textures\\synthetic\\diffuse.dds";
      textureSet->textures[1].get() = "textures\\synthetic\\normal.dds";
      shader->SetAlpha(0.5f);
      shader->SetDoubleSided(true);
      auto alpha = std::make_unique<NiAlphaProperty>();
      alpha->flags = 4845;
      alpha->threshold = 42;
      shape->AlphaPropertyRef()->index = hdr.AddBlock(std::move(alpha));
      auto root = nif.GetRootNode();
      root->transform.translation = Vector3(10, 20, 30);
      root->transform.rotation = Matrix3(0, -1, 0, 1, 0, 0, 0, 0, 1);
      root->transform.scale = 2;
      auto child = std::make_unique<NiNode>();
      auto childPtr = child.get();
      child->name.get() = "Parent";
      child->transform.translation = Vector3(1, 0, 0);
      auto childId = hdr.AddBlock(std::move(child));
      root->childRefs.AddBlockRef(childId);
      nif.SetParentNode(shape, childPtr);
      if (mode == "mirrored")
        shape->transform.scale = -1;
      if (mode == "singular")
        shape->transform.scale = 0;
      if (mode == "shear") {
        root->transform.Clear();
        childPtr->transform.Clear();
        shape->transform.rotation = Matrix3(2, 0, 0, 0, 1, 0, 0, 0, 1);
      }
      if (mode == "bad-index") {
        triangles[0].p3 = 9;
        shape->SetTriangles(triangles);
      }
      if (mode == "nan")
        shape->transform.translation.x =
            std::numeric_limits<float>::quiet_NaN();
      if (mode == "cycle")
        childPtr->childRefs.AddBlockRef(nif.GetBlockID(root));
      if (mode == "dangling")
        childPtr->childRefs.AddBlockRef(99999);
      if (mode == "skinned") {
        shape->SetSkinned(true);
        shader->SetSkinned(true);
      }
      if (mode == "unsupported")
        hdr.AddBlock(std::make_unique<NiIntegerExtraData>());
      if (mode == "bad-utf8")
        shape->name.get() = std::string("\xc0\x80", 2);
      if (mode == "bad-alpha")
        shader->SetAlpha(2);
      if (mode == "duplicate-parent")
        root->childRefs.AddBlockRef(nif.GetBlockID(shape));
      if (mode == "uv-transform") {
        auto lighting = dynamic_cast<BSLightingShaderProperty *>(shader);
        lighting->uvScale = Vector2(2, 3);
        lighting->uvOffset = Vector2(0.25f, 0.5f);
      }
      if (mode == "output-limit") {
        textureSet->textures.resize(16);
        for (auto &path : textureSet->textures)
          path.get() = std::string(4096, '\x01');
        for (int i = 0; i < 180; ++i) {
          auto duplicate = nif.CreateShapeFromData(
              "Repeated material", &vertices, &triangles, &uvs, &normals);
          duplicate->ShaderPropertyRef()->index = nif.GetBlockID(shader);
        }
      }
      if (mode == "deep") {
        auto last = childPtr;
        for (int i = 0; i < 260; ++i) {
          auto node = std::make_unique<NiNode>();
          auto ptr = node.get();
          last->childRefs.AddBlockRef(hdr.AddBlock(std::move(node)));
          last = ptr;
        }
        nif.SetParentNode(shape, last);
      }
      if (mode == "many-vertices") {
        for (int i = 0; i < 4; ++i)
          nif.CreateShapeFromData("Large", &vertices, &triangles, &uvs,
                                  &normals);
      }
      NifSaveOptions options;
      options.optimize = false;
      options.sortBlocks = false;
      auto path = std::filesystem::path(argv[1]) /
                  ((sse ? "sse-" : "le-") + mode + ".nif");
      if (nif.Save(path, options) != 0)
        throw std::runtime_error("fixture save failed");
    }
  }
}
