# Changelog

## [1.2.0](https://github.com/Project-Colony/Eidos/compare/v1.1.0...v1.2.0) (2026-08-03)


### Features

* support non-Bethesda games, starting with Stellar Blade ([#5](https://github.com/Project-Colony/Eidos/issues/5)) ([4e50858](https://github.com/Project-Colony/Eidos/commit/4e50858d5a05342889936f1e56e5b3758f2f4276))

## [1.1.0](https://github.com/Project-Colony/Eidos/compare/v1.0.2...v1.1.0) (2026-08-01)


### Features

* **gui:** let the game's own content be ordered like any other row ([9f35576](https://github.com/Project-Colony/Eidos/commit/9f3557646584c90eaa0ecf554fd36aaf6d9dcd06))


### Fixes

* **ci:** let the release job start the build it asks for ([604a0cd](https://github.com/Project-Colony/Eidos/commit/604a0cd33505cc799f0b36afd91f5977c3622c54))
* **gui:** separators could not be reordered at all ([0a4ff4b](https://github.com/Project-Colony/Eidos/commit/0a4ff4b10def21e03de85b2ec673bad6cf9b0d4e))

## [1.0.2](https://github.com/Project-Colony/Eidos/compare/v1.0.1...v1.0.2) (2026-07-31)


### Fixes

* **ci:** attach artifacts to a release that already exists ([9caf1f1](https://github.com/Project-Colony/Eidos/commit/9caf1f18c2e97415fd4a615271cc078b74180498))
* **ci:** stop naming a component this repo does not have ([12ea5a2](https://github.com/Project-Colony/Eidos/commit/12ea5a2983c3676978bd24c8012b2ca72065d3ae))

## [1.0.1](https://github.com/Project-Colony/Eidos/compare/v1.0.0...v1.0.1) (2026-07-31)


### Fixes

* **ci:** release-please cannot read a version the crates inherit ([bc28968](https://github.com/Project-Colony/Eidos/commit/bc2896844c124192ef35ac334f79fe9b2d983cc1))
* **ci:** stop assuming a Cargo package lives at the workspace root ([18a3acb](https://github.com/Project-Colony/Eidos/commit/18a3acbd26df16c2ce3d272caff2a13ea118612c))
* **ci:** sync Cargo.lock on a condition that actually holds ([b94cd90](https://github.com/Project-Colony/Eidos/commit/b94cd909ac571f68bd8aeb8ce58251b20d1b1a1b))
* **ci:** tag the version line so release-please can find it ([a4531f1](https://github.com/Project-Colony/Eidos/commit/a4531f11f5850526b77e7b55b29884d9fe14008e))
* **ci:** use the command that actually rewrites Cargo.lock ([86705be](https://github.com/Project-Colony/Eidos/commit/86705be8a63040703004379d965a93aaefb9ac9f))

## 1.0.0

Skyrim SE has been played through Eidos daily for weeks - SKSE, script-extender
preloaders, Creation Club, LOOT-sorted load orders, per-profile saves, tools
generating LOD and bodies into the Overwrite. Nothing in the design is
provisional any more, so the version stops pretending it is.

### Correctness

Every one of these failed *silently*. That is what made them worth the release.

* **A mod spelling one directory two ways lost everything under the second.**
  ext4 keeps `meshes/` and `Meshes/` apart; the merged view must not, and real
  mods ship both. The resolver committed to the exact-case match for a component
  and abandoned the whole layer when the rest of the path was not underneath it.
  On a real 50-layer instance that hid 74 files of one mod, with no error
  anywhere ([#e606a65]).
* **The game path in the Wine prefix stopped being re-registered** once anything
  else overwrote it. The game's own 32-bit launcher rewrites that key through
  whatever drive letter Wine offers, and Steam then moves the letter - leaving a
  value that was correct when written and resolved to nothing afterwards. Tools
  died citing a directory that does not exist and naming neither Eidos nor the
  cause.
* **xEdit's QuickAutoClean ran without the compatibility mode** the rest of the
  family gets. It is the executable users actually run, since cleaning the
  official masters is a prerequisite of DynDOLOD and most load-order guides.
* **Clearing the Overwrite or turning it into a mod took no instance lock**, so
  it could run against an instance another process was using.

### DynDOLOD, end to end

DynDOLOD's LOD generator is routed to Wine's Mono, whose `System.Uri` initialiser
calls a method Mono does not implement: it dies before its first line of work and
leaves a log holding a version banner and nothing else. Installing .NET Framework
does not help, because Proton replaces the loader that would find it on every
prefix update.

Eidos now provisions the modern .NET runtime that build needs, as a third tier of
prerequisite - fetched once, checksum-verified against a value compiled into the
binary, and shared by every instance.

### Nexus

* OAuth: the client authenticates with an access token wherever it accepted an
  API key, choosing between them from what is stored and renewing a stale
  session. Signing in still waits on a `client_id` from Nexus; everything under
  it is here and tested.
* The account tier is always spelled out. "(Premium)" or nothing made a free
  account indistinguishable from one whose tier had not been checked - and the
  difference decides whether a download link can be fetched at all.

### Speed

A save that took twenty seconds to load takes six to seven, and cell changes are
immediate.

| | before | now |
|---|---|---|
| directory reads in one session | 5,608,084 | 464,564 |
| `opendir` | 516,301 | 1 |
| `readdir` | 799 ms | 105 ms |

Measured on a real instance played normally. `index_health`, `index_agrees` and
`listing_cost` reproduce every figure on your own setup, and `index_agrees` is
what caught the case-folding bug above.

### Interface

* The LOOT report no longer closes when you click it, and copies whole to the
  clipboard - it is a worklist you read off while xEdit runs on another screen.
* Each tool prerequisite shows its real state, and the missing ones are buttons.
* The window has a desktop identity, so it has an icon in your taskbar.
* Eidos has a mark: a fragmented E resolving into a lozenge - the layers becoming
  one view, and *eidos*, the Form.

### Documentation

Reorganised by who is reading: `guide/` to use it, `internals/` to read the code,
`project/` for why it exists. The README is a front page again.

---

Earlier releases: [0.5.1](https://github.com/Project-Colony/Eidos/releases/tag/v0.5.1),
[0.5.0](https://github.com/Project-Colony/Eidos/releases/tag/v0.5.0).

[#e606a65]: https://github.com/Project-Colony/Eidos/commit/e606a65
