# Eidos documentation

Sorted by who is reading, not by subject.

## Using Eidos

| | |
|---|---|
| [guide/install.md](guide/install.md) | getting it on your machine and pointing Steam at it |
| [guide/usage.md](guide/usage.md) | the CLI and the GUI, end to end |
| [guide/tools.md](guide/tools.md) | xEdit, BodySlide, DynDOLOD: adding them, and the runtimes their name selects |
| [guide/fallout4.md](guide/fallout4.md) | Fallout 4: why it needs no launch hack, the version branches, the NVIDIA weapon-debris crash |
| [guide/graphics.md](guide/graphics.md) | Community Shaders, DLSS, frame generation: the launch option and the silent killers |
| [guide/troubleshooting.md](guide/troubleshooting.md) | when something looks wrong: switches, counters, known issues |
| [guide/extensions.md](guide/extensions.md) | adding your own entries: previews, save info, installers, as out-of-process programs |
| [guide/examples/extensions/](guide/examples/extensions/README.md) | the extension protocol contract, with runnable example manifests |

## Reading the code

| | |
|---|---|
| [internals/architecture.md](internals/architecture.md) | why FUSE, the crate map and its process boundaries, the daemon's design, caching, write semantics |
| [internals/performance.md](internals/performance.md) | what was slow, what each change bought, how to measure it yourself |
| [internals/paths.md](internals/paths.md) | where Eidos keeps its files, and how an upgrade moves them without losing any |
| [internals/integrity.md](internals/integrity.md) | how installs, collections and generated output stay consistent when something fails |
| [internals/contributing.md](internals/contributing.md) | building, testing, and where everything lives |
| [internals/adding-games.md](internals/adding-games.md) | wiring a new game family |
| [internals/packaging.md](internals/packaging.md) | distribution, and why the obvious formats do not work |

## Why it exists

| | |
|---|---|
| [project/landscape.md](project/landscape.md) | the problem, every Linux approach, what is exclusive here |
| [project/status.md](project/status.md) | the full done/remaining ledger |
| [project/game-support.md](project/game-support.md) | what it takes to install mods for games beyond Bethesda |

Security policy and how to report a vulnerability: [../SECURITY.md](../SECURITY.md).
