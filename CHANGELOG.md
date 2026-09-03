# Changelog

## [1.14.1](https://github.com/Project-Colony/Eidos/compare/v1.14.0...v1.14.1) (2026-09-03)


### Performance

* **build:** one codegen unit, for a 22% smaller binary ([#50](https://github.com/Project-Colony/Eidos/issues/50)) ([ebe43a1](https://github.com/Project-Colony/Eidos/commit/ebe43a15df30f2f74bebf291c165d7c0234596ff))

## [1.14.0](https://github.com/Project-Colony/Eidos/compare/v1.13.0...v1.14.0) (2026-09-03)


### Features

* **install:** extract on a worker thread, with a progress dialog ([#48](https://github.com/Project-Colony/Eidos/issues/48)) ([4c2c7b0](https://github.com/Project-Colony/Eidos/commit/4c2c7b066c7076a551d22bdba14352b01b021dc9))

## [1.13.0](https://github.com/Project-Colony/Eidos/compare/v1.12.0...v1.13.0) (2026-09-02)


### Features

* **gamefeatures:** warn when the prefix's C++ runtime is split ([#46](https://github.com/Project-Colony/Eidos/issues/46)) ([c0d726e](https://github.com/Project-Colony/Eidos/commit/c0d726ed73f30d006254252540ec912d0d3cdbd8))


### Fixes

* **nexus:** keep the OAuth secrets out of Debug output ([#45](https://github.com/Project-Colony/Eidos/issues/45)) ([54cad22](https://github.com/Project-Colony/Eidos/commit/54cad229d1c8c19228d9dfff848384e1c09857f3))

## [1.12.0](https://github.com/Project-Colony/Eidos/compare/v1.11.2...v1.12.0) (2026-08-26)


### Features

* **gui:** animation, Preferences to the Colony convention, and the 57 shared palettes ([#41](https://github.com/Project-Colony/Eidos/issues/41)) ([7f71b25](https://github.com/Project-Colony/Eidos/commit/7f71b2545b470040ecc11cffbb9df8ebedeec86f))
* **gui:** drag the divider to resize the two panes ([#40](https://github.com/Project-Colony/Eidos/issues/40)) ([97c68e5](https://github.com/Project-Colony/Eidos/commit/97c68e566931f2d624f7f194b125a1241e19ebf4))

## [1.11.2](https://github.com/Project-Colony/Eidos/compare/v1.11.1...v1.11.2) (2026-08-25)


### Internals

* **docs:** one directory per language, mirroring the repo root ([#38](https://github.com/Project-Colony/Eidos/issues/38)) ([bfa2998](https://github.com/Project-Colony/Eidos/commit/bfa299862b1e9dbcb3124edfb374e1e141081eea))

## [1.11.1](https://github.com/Project-Colony/Eidos/compare/v1.11.0...v1.11.1) (2026-08-25)


### Fixes

* **instance:** the lock table and the kernel could disagree, and it failed a release ([#36](https://github.com/Project-Colony/Eidos/issues/36)) ([63735b4](https://github.com/Project-Colony/Eidos/commit/63735b4a4b39f979b919fdd76a1f7d934dfc73bd))

## [1.11.0](https://github.com/Project-Colony/Eidos/compare/v1.10.0...v1.11.0) (2026-08-25)


### Features

* **docs:** translate the player-facing pages, and make a stale one impossible to ignore ([#34](https://github.com/Project-Colony/Eidos/issues/34)) ([1fa2a06](https://github.com/Project-Colony/Eidos/commit/1fa2a0640a09ae8c3b77108a9e31c6cad7329a45))


### Fixes

* **gamedef:** let a user game declare its own tools, and correct two stale accounts ([#33](https://github.com/Project-Colony/Eidos/issues/33)) ([65ab4bd](https://github.com/Project-Colony/Eidos/commit/65ab4bdb223ed9aa84a69049cc7455dff3595f42))

## [1.10.0](https://github.com/Project-Colony/Eidos/compare/v1.9.1...v1.10.0) (2026-08-25)


### Features

* **gui:** watch Eidos's own machinery, not only the user's mods ([#29](https://github.com/Project-Colony/Eidos/issues/29)) ([3a5a0bf](https://github.com/Project-Colony/Eidos/commit/3a5a0bf4c69c29adab7547fc9eaa456d76b38868))


### Fixes

* **gui:** main was red on the pinned toolchain, on a lint the newer one drops ([#32](https://github.com/Project-Colony/Eidos/issues/32)) ([55bd10a](https://github.com/Project-Colony/Eidos/commit/55bd10a99a84ebaf624c9fba80822348fe11f824))
* **install:** an archive that used Windows path separators installed as a flat pile ([#28](https://github.com/Project-Colony/Eidos/issues/28)) ([a34f4c9](https://github.com/Project-Colony/Eidos/commit/a34f4c9d7da9c1b4183597f4821088c146d76217))
* **proton:** the prefix's S: drive belongs on the library root, not on steamapps ([#30](https://github.com/Project-Colony/Eidos/issues/30)) ([bd9f897](https://github.com/Project-Colony/Eidos/commit/bd9f8975f732c9fc4af963a6b5d05341030ea20f))

## [1.9.1](https://github.com/Project-Colony/Eidos/compare/v1.9.0...v1.9.1) (2026-08-25)


### Fixes

* **tools:** a tool started from the GUI got neither its profile nor a usable game path ([#26](https://github.com/Project-Colony/Eidos/issues/26)) ([115f065](https://github.com/Project-Colony/Eidos/commit/115f0657e19c717f5d19a5a687795f7697c92b61))

## [1.9.0](https://github.com/Project-Colony/Eidos/compare/v1.8.2...v1.9.0) (2026-08-24)


### Features

* **tools:** find xEdit and its QuickAutoClean twin instead of waiting to be told ([0752772](https://github.com/Project-Colony/Eidos/commit/075277234a467411e3c1c6392c0333377934aa16))

## [1.8.2](https://github.com/Project-Colony/Eidos/compare/v1.8.1...v1.8.2) (2026-08-24)


### Fixes

* **gui:** renaming a mod sent it to the top of the list, disabled ([0368347](https://github.com/Project-Colony/Eidos/commit/03683478f1e0c9bb1e4c3d82353ff41c76613551))
* **loot:** the sort never used record overlap, so a dragged plugin stayed put ([c5fb1fe](https://github.com/Project-Colony/Eidos/commit/c5fb1fe9e256f492bb0676dcfaa3ca6aaaf80035))
* **nexus:** a stored sign-in was rejected on every launch, by an endpoint that cannot accept it ([1a6012d](https://github.com/Project-Colony/Eidos/commit/1a6012d98a8bb36ce17ce4651d028500e784b66e))

## [1.8.1](https://github.com/Project-Colony/Eidos/compare/v1.8.0...v1.8.1) (2026-08-24)


### Documentation

* the eleven parity features, collections, and where files live now ([98988e8](https://github.com/Project-Colony/Eidos/commit/98988e8d95a554e28b5f28b67a974cb34191f75d))

## [1.8.0](https://github.com/Project-Colony/Eidos/compare/v1.7.1...v1.8.0) (2026-08-24)


### Features

* **addons:** user extensions, as manifests rather than loaded code ([7eb28c1](https://github.com/Project-Colony/Eidos/commit/7eb28c17b7d3b8867372b473c8011b65e34f5872))
* **downloads:** pause, resume and cancel a transfer ([890cc8c](https://github.com/Project-Colony/Eidos/commit/890cc8c844118e0cc9514b2dea7be44490fce98a))
* **gui:** a File menu listing every folder that matters ([b313631](https://github.com/Project-Colony/Eidos/commit/b313631eca9f5918c16155ef3ba00275c6840be8))
* **gui:** a log pane, reading the files that actually hold the answers ([89c6baa](https://github.com/Project-Colony/Eidos/commit/89c6baa309b49a951aee49ecf7bc5b4814977dd8))
* **gui:** a page link for mods that are not on Nexus ([09ef093](https://github.com/Project-Colony/Eidos/commit/09ef093b00ea34f08f39b7064962daccb13e4458))
* **gui:** an Archives tab that says why each BSA/BA2 does or does not load ([d15190b](https://github.com/Project-Colony/Eidos/commit/d15190bb5cf2f9be81934f0b6b6eb9248e8cfd97))
* **gui:** an INI editor, on the copy that actually persists ([eb43ed2](https://github.com/Project-Colony/Eidos/commit/eb43ed218508f59b35d76168e02c8b7b9ba896d6))
* **gui:** an instance manager, and a fix for every atomic write in the tree ([788565c](https://github.com/Project-Colony/Eidos/commit/788565cd5801d92c39cd29952159b6d289f8c054))
* **gui:** back a mod up before touching it, and restore it after ([a3f0361](https://github.com/Project-Colony/Eidos/commit/a3f03619bba81454379ebdc48942776fe9210119))
* **gui:** backups, a plugin context menu, and MD5 recovery for stray archives ([19e1235](https://github.com/Project-Colony/Eidos/commit/19e12357a77620edbae1229ec32a34b8e44b6264))
* **gui:** collapse others, hover-to-expand, and the four mouse-keyboard gestures ([71dceb9](https://github.com/Project-Colony/Eidos/commit/71dceb961a8239a88999f477c6a43ed563ba0fdc))
* **gui:** colour any mod, and surface its note on the row ([7964deb](https://github.com/Project-Colony/Eidos/commit/7964deb882830799c9da89e547317815b8340252))
* **gui:** editable categories, and six filter fixes ([d4d2650](https://github.com/Project-Colony/Eidos/commit/d4d26504329b4008a896b50896417c1d1af95c8f))
* **gui:** eight optional columns, and sorting by any of them ([23a9ac2](https://github.com/Project-Colony/Eidos/commit/23a9ac23b138013c619bfb765f9e7679ffe7c36b))
* **gui:** export the mod list from the window, with scope and columns ([f4afbed](https://github.com/Project-Colony/Eidos/commit/f4afbed46ff1346dc8a38ee8269fab9132721fe6))
* **gui:** filter the mod list by state, not just by name ([ba6a640](https://github.com/Project-Colony/Eidos/commit/ba6a6408a2ca939fe9a1463dd54b343040b87092))
* **gui:** group the list by category or by source ([010a344](https://github.com/Project-Colony/Eidos/commit/010a34465e4b1472bea3943a6657a1cec62fe83f))
* **gui:** highlight the plugins the selected mod ships ([320b7c6](https://github.com/Project-Colony/Eidos/commit/320b7c6607daa3c4f8a97b45c703047c46e79af2))
* **gui:** install and create at a chosen position, and bulk enable/disable ([7dc1181](https://github.com/Project-Colony/Eidos/commit/7dc11815e84fde06d9c0d1df0c8ef32bb929c65a))
* **gui:** install by dragging, from the Downloads list and from the desktop ([9d39d01](https://github.com/Project-Colony/Eidos/commit/9d39d0162a9a0662e0ef30084490ed15c9ea6e9f))
* **gui:** make the Data tab read the real union, and give it depth ([09c4504](https://github.com/Project-Colony/Eidos/commit/09c45046ed97839fe4667758210ef71688a6f02a))
* **gui:** new folder, rename, delete and open in a mod's file tree ([bfe5538](https://github.com/Project-Colony/Eidos/commit/bfe5538da527f06b276d15015121c64c7688f633))
* **gui:** preview images and text from the Data tab and a mod's file tree ([ca0d1df](https://github.com/Project-Colony/Eidos/commit/ca0d1df8bf498155898c4924fb75ccce3cf39cb3))
* **gui:** save screenshots, multi-select, transfer between profiles, and a watcher ([e784081](https://github.com/Project-Colony/Eidos/commit/e78408102c16555ce79e754fa6a5dbac8e3eb326))
* **gui:** send Overwrite files back to the mods that provide them ([e85633e](https://github.com/Project-Colony/Eidos/commit/e85633ec64e4f0e10757917fc5e227d9e01dfe88))
* **gui:** send plugins to an exact index, and show what is left of the Nexus budget ([0d7a28d](https://github.com/Project-Colony/Eidos/commit/0d7a28d06115cfe5121633d417311ba946e3e81d))
* **gui:** show that a drop will be taken, and say when it cannot be ([9282051](https://github.com/Project-Colony/Eidos/commit/928205103c35413b2181dd82a51ddeb2c3cfba93))
* **gui:** the Downloads tab as an archive library, not a transfer queue ([d842f32](https://github.com/Project-Colony/Eidos/commit/d842f321df99a08fbdb6a861c302a8ec139a26cc))
* **gui:** the two state flags, and MO2's own key for silencing them ([a1cac39](https://github.com/Project-Colony/Eidos/commit/a1cac3917db468b5d9a85cb6e77099450bc8098b))
* **nexus:** collections, as a browser rather than an installer ([c9ba2c0](https://github.com/Project-Colony/Eidos/commit/c9ba2c025b74a39a0e517c73c2a15923059ecd4d))
* **nexus:** flag mods whose Nexus page has been taken down ([f33a6e8](https://github.com/Project-Colony/Eidos/commit/f33a6e80721cc29741cbc1cfed1f1321642542e8))
* **nexus:** offline mode, and a CDN order that is an order not a filter ([837de00](https://github.com/Project-Colony/Eidos/commit/837de008b62490475557e33144eec69a67a62705))
* **nexus:** record who made a mod, and link to their profile ([0d775e0](https://github.com/Project-Colony/Eidos/commit/0d775e0e3e06ff4f11207abae867fa1291e95d73))
* **paths:** the Colony layout, and stop the tests eating your preferences ([eefd0eb](https://github.com/Project-Colony/Eidos/commit/eefd0eb1929a8bcd27417ffd4cb763360ecd4dbc))
* **tools:** capture a tool's output into a mod instead of the Overwrite ([160a399](https://github.com/Project-Colony/Eidos/commit/160a3993e5cbb4a63be09128cff6d26809fb9f26))
* **tools:** Steam AppID, hide, pin, and a desktop shortcut ([2852f8b](https://github.com/Project-Colony/Eidos/commit/2852f8b04b067c39b7b10a14b69834a56fa38ca4))


### Fixes

* 35 defects found reviewing the eight new features ([e260224](https://github.com/Project-Colony/Eidos/commit/e260224e36079e90f040bb7b5c87d1a771fdc8d2))
* 41 defects found reviewing the fourteen features ([7468d84](https://github.com/Project-Colony/Eidos/commit/7468d84a7356901a99e094c8d5ab654c49ed9eac))
* **cli:** a download link is not a log rotation bucket ([888c5d1](https://github.com/Project-Colony/Eidos/commit/888c5d1fa536e246a51cca6fee3ac935ee89acdc))
* **gui:** a confirmation carries a name, not a position in a list ([40f78f8](https://github.com/Project-Colony/Eidos/commit/40f78f8a6947dd89e920100bce49208a7aa04037))
* **gui:** one answer to "which rows is the list drawing", and everything asks it ([9048519](https://github.com/Project-Colony/Eidos/commit/9048519dda91cfe7ce7c71ac6a46b71eaf1a72a0))
* **gui:** the collection pane's fetch button was an unmetered installer ([dfce933](https://github.com/Project-Colony/Eidos/commit/dfce9330d92cd6b0b8db2d09206b02546ec843ef))
* **log:** bound the log directory, and stop the sweep eating the wrong files ([90d9bd3](https://github.com/Project-Colony/Eidos/commit/90d9bd36d758f809cd428b655fda5446ee42c6d3))
* **nexus:** a mod knows who made it from the moment it lands ([c01beb5](https://github.com/Project-Colony/Eidos/commit/c01beb54712fdb4cb233e771fb8179b5f70282ff))
* the second review pass, including two defects in the first pass's fix ([05ac149](https://github.com/Project-Colony/Eidos/commit/05ac14912473302ef97260465f5b05001bc582a0))


### Performance

* **gui:** stop reading the filesystem on every frame ([61766f0](https://github.com/Project-Colony/Eidos/commit/61766f09f775805da8978e033476b49847908cdd))


### Documentation

* cover output capture, drag-install, download controls, and the new panes ([b2be006](https://github.com/Project-Colony/Eidos/commit/b2be0068c4e28126fb0110fce05c1bada04844ff))
* **meta:** set_uploader keeps the name when the URL is unusable ([0a683d4](https://github.com/Project-Colony/Eidos/commit/0a683d4a3e9b91471e56fd2ca382f0facd573316))

## [1.7.1](https://github.com/Project-Colony/Eidos/compare/v1.7.0...v1.7.1) (2026-08-22)


### Fixes

* **nexus:** read the account from the token, not from users/validate ([fad3c1f](https://github.com/Project-Colony/Eidos/commit/fad3c1fb3c0edb1a6a5fda87717b2bd9429b7f93))

## [1.7.0](https://github.com/Project-Colony/Eidos/compare/v1.6.0...v1.7.0) (2026-08-22)


### Features

* **nexus:** sign in under Eidos's own registered client id ([df494cd](https://github.com/Project-Colony/Eidos/commit/df494cdf5dd0de78db834a9964575a8cbb86b3f3))

## [1.6.0](https://github.com/Project-Colony/Eidos/compare/v1.5.0...v1.6.0) (2026-08-20)


### Features

* read Fallout 4 saves, and stop shifting every save's timestamp ([0d473fa](https://github.com/Project-Colony/Eidos/commit/0d473fa990ac633bfa06b00d7f6ce23da93b8844))

## [1.5.0](https://github.com/Project-Colony/Eidos/compare/v1.4.1...v1.5.0) (2026-08-18)


### Features

* portable instances are first-class - registry, CLI paths, GUI reopen ([3ebf75e](https://github.com/Project-Colony/Eidos/commit/3ebf75e592db2137c0ed44f3e6d4e96b2011b184))
* refuse an instance inside a game's install folder ([9f88404](https://github.com/Project-Colony/Eidos/commit/9f884049be8696d3a0e4db3652decc98f3afbe00))

## [1.4.1](https://github.com/Project-Colony/Eidos/compare/v1.4.0...v1.4.1) (2026-08-17)


### Documentation

* a Steam launch options section in the README ([cd6d35a](https://github.com/Project-Colony/Eidos/commit/cd6d35abf9bd63f0db3774e92c12df5bae80bed3))
* Community Shaders DLSS and frame generation under Eidos ([61f414b](https://github.com/Project-Colony/Eidos/commit/61f414ba276ab06ca2019bab30a389db6f039a62))
* name the final launch command for a CS/DLSS setup outright ([5aacaa1](https://github.com/Project-Colony/Eidos/commit/5aacaa1beb5f3ed29c3a40c9013a65a1ef6a6799))

## [1.4.0](https://github.com/Project-Colony/Eidos/compare/v1.3.0...v1.4.0) (2026-08-17)


### Features

* **nexus:** remove personal API keys, sign in with OAuth only ([#7](https://github.com/Project-Colony/Eidos/issues/7)) ([cdd3219](https://github.com/Project-Colony/Eidos/commit/cdd321990e0bf9a81b5f4d846905125b42dc942b))


### Fixes

* correctness defects, a read-shape survey, and every dependency up to date ([#15](https://github.com/Project-Colony/Eidos/issues/15)) ([725d6d4](https://github.com/Project-Colony/Eidos/commit/725d6d4ed74639c0d6ef15757cb4deffb56d060b))
* **gui:** eleven verified defects from a full read of the remaining GUI, and LOOT plugin flags ([46abd31](https://github.com/Project-Colony/Eidos/commit/46abd317185cbfeafdf871d9283da81251b72311))
* thirteen defects found by a crate-by-crate sweep of all seventeen crates ([a4778c2](https://github.com/Project-Colony/Eidos/commit/a4778c2f4995e9acb4c638e3fc86d883e0c45d52))

## [1.3.0](https://github.com/Project-Colony/Eidos/compare/v1.2.0...v1.3.0) (2026-08-04)


### Features

* **gui:** MO2 parity for conflicts, dragging, and a Colony-shaped Settings ([#8](https://github.com/Project-Colony/Eidos/issues/8)) ([9aad5f7](https://github.com/Project-Colony/Eidos/commit/9aad5f749c31f9105923d41bd1c6f257f6056660))


### Performance

* **core:** index the overwrite, taking directory ops out of resolution ([#13](https://github.com/Project-Colony/Eidos/issues/13)) ([8ce6821](https://github.com/Project-Colony/Eidos/commit/8ce68211e23932798ca70022d6a089f64912011b))


### Internals

* **cli:** one module per subcommand ([#10](https://github.com/Project-Colony/Eidos/issues/10)) ([1a65d2f](https://github.com/Project-Colony/Eidos/commit/1a65d2f4f707dac7063c9b61a9f7959da89f777c))
* **fuse:** split the daemon into modules by role ([#9](https://github.com/Project-Colony/Eidos/issues/9)) ([484b894](https://github.com/Project-Colony/Eidos/commit/484b894669c1e51e8b7e46e8f51653e744deb846))
* **install:** make install.rs a directory module ([#11](https://github.com/Project-Colony/Eidos/issues/11)) ([808f36e](https://github.com/Project-Colony/Eidos/commit/808f36e4dda6c29e8af136e9ba6fc124068e9ec2))
* **instance:** make profile.rs a directory module, split by method ([#12](https://github.com/Project-Colony/Eidos/issues/12)) ([730311d](https://github.com/Project-Colony/Eidos/commit/730311db961f2878ed2740104d8d99f96a3029ed))

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
