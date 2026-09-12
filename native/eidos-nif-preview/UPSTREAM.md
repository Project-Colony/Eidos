# nifly source and licenses

This helper builds the unmodified `include/`, `src/` and `external/` source directories from
[ousnius/nifly commit f445ebc2e6ace115bbc2de56c70c3562914c5c15](https://github.com/ousnius/nifly/tree/f445ebc2e6ace115bbc2de56c70c3562914c5c15).
The upstream `LICENSE` and `README.md`, including its credits, are retained in `vendor/nifly/`.
Upstream identifies its version as 1.0.0; the commit is the reproducibility pin.

Downloaded archive: `https://codeload.github.com/ousnius/nifly/tar.gz/f445ebc2e6ace115bbc2de56c70c3562914c5c15`

Archive SHA-256: `18c63431457e3793e0002bf0a8f7eecd7d77876507afb210ea0ffb3bdac8fff8`.

- nifly: GPL version 3; see the complete `vendor/nifly/LICENSE` and source headers.
- `external/half.hpp`: Christian Rau, MIT license retained in the header (actual bundled version 2.2.1).
- `external/Miniball.hpp`: Bernd Gaertner, GPL version 3 or later, complete notice retained in the header.
- Upstream credits ousnius, jonwd7, Caliente and NifTools contributors; retain its README credits when distributing source.

No Catch2 code, upstream test assets or game models are vendored. Eidos uses its own CMake target and Python acceptance test; there are no downloads during the build. Distributing the helper requires preserving these notices and providing its corresponding source under the applicable GPL terms. The release packager owns that packaging step.

The helper deliberately does not call `NifFile::Load`: that convenience method calls `PrepareData`, including triangle cleanup. It calls nifly's established `NiHeader` and block factory readers on individual bounded block streams, then validates graph references and geometry before rendering output. Unsupported block bytes are skipped using the header's validated sizes. No vendored parser code is patched.
