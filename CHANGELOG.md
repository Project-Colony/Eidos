# Changelog

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
