# Where Eidos keeps its files

Eidos follows the Colony filesystem layout: one `Colony/Eidos/` pair under each
of the three platform roots. The roots come from `colony_ui::paths::locate`, the
ecosystem's single answer to the question; `crates/eidos-paths` only names the
sub-directories Eidos uses inside them, and no other crate builds a path from
`$XDG_*` or `dirs` itself.

| Root | Path | What lives there |
|---|---|---|
| config | `~/.config/Colony/Eidos/` | `settings.ini`, `nexus.ini` (the Nexus session, mode 0600), `instances.ini`, `games/*.toml`, `addons/*.toml` |
| data | `~/.local/share/Colony/Eidos/` | `instances/<game>/` (global instances), `runtimes/` (downloaded tool runtimes), `logs/` (session logs) |
| cache | `~/.cache/Colony/Eidos/` | `loot/<repo>/` (LOOT masterlist and prelude, fetched again when missing) |

A relative `$XDG_*_HOME` is ignored, as the XDG specification requires, and so
is a relative `$HOME`: under Proton the working directory is the game's folder,
and honouring a relative value would put the user's settings there. With no
usable home at all, the roots are the ones a `$HOME` of `/` would give
(`/.config/Colony/Eidos/` and so on), where writing fails and nothing is saved.
Never the shared temp directory, where another local user could create the
folder first and plant definitions in it.

Two things stay outside the Colony tree on purpose:

- **Portable instances** live wherever the user put them. They are never moved.
- **`.desktop` launchers** (tool shortcuts and the `nxm://` handler) go to
  `~/.local/share/applications/`, where the desktop looks for them
  (`eidos_paths::desktop_entries_dir`).

Inside an instance, `loot/` keeps only what belongs to that instance: the
user's `userlist.yaml` and the case bridge built from its mods.

## Moving off the old layout

Up to 1.18.4, Eidos used other paths. The first launch of a newer version
carries them across:

| Old path | New path | How |
|---|---|---|
| `~/.config/eidos/` | `~/.config/Colony/Eidos/` | copied |
| `~/.local/state/Colony/Eidos/logs/` (1.18) or `~/.local/state/eidos/logs/` | `~/.local/share/Colony/Eidos/logs/` | copied |
| `~/.local/share/eidos/runtimes/` | `~/.local/share/Colony/Eidos/runtimes/` | hard-linked |
| `~/.local/share/eidos/<game>/` | `~/.local/share/Colony/Eidos/instances/<game>/` | hard-linked, then repointed |
| `<instance>/loot/masterlist.yaml`, `prelude.yaml` | `~/.cache/Colony/Eidos/loot/<repo>/` | not moved: fetched again |

Every move follows the same rule (`eidos_paths::migrate_tree`):

1. Build the copy in a staging directory beside the destination
   (`.<name>.migrating-<pid>`).
2. Write the marker `.migrated-from` into the copy, then rename the copy into
   place. The new directory appears complete, or not at all.
3. Never remove, rename or write to the old directory. It stays exactly as it
   was, so an older Eidos still finds it and a wrong migration loses nothing.
4. Until the marker is down, every resolver (`config_dir`, `logs_dir`,
   `runtimes_dir`, `global_instance_dir`) returns the old directory while it
   exists. A copy that fails (a read-only or full disk) leaves no marker, so that
   session keeps using the old files and the next launch tries again.
5. A destination that already exists without a marker (left by 1.18 when its
   copy failed half way) is filled in: missing files are added, nothing is
   overwritten.

A symlink is recreated pointing at the same place, never followed. A relative
link that would climb out of the tree is made absolute so it still reaches the
same target from its new folder.

The migration runs at startup in both binaries, before the session log opens
and before anything reads a setting, and its notes are written to that log.

### Global instances

A global instance is tens of gigabytes, so it is hard-linked rather than copied
(`Carry::Link`): the move is near instant and takes no extra space. Each side
keeps its own directory entries, so deleting a file on one side never removes it
from the other. Eidos writes its own files by replacing them, which leaves the
old side as it was; a file a game edits in place through the merged view
changes on both sides, since both name the same data.

`eidos_transfer::migrate_global_instances` adds three things around the move:

- It takes the instance lock on the old folder first. If a game started by an
  older Eidos is still running from it, the instance is left for a later launch
  rather than linked across mid-write. Taking the lock creates the old
  folder's `.eidos.lock` if it is missing, as opening the instance in any
  Eidos does; that file is the only thing the move writes there.
- It runs the relocation pass that `eidos unpack` uses, so `tools.ini` entries
  and each mod's `installationFile` point at the new folder. Once that pass runs
  clean it leaves a second marker, `.migrated-repointed`. Until then (the pass
  hit a file it could not rewrite, or the launch was killed between the move and
  the pass), every launch runs it again on the moved instance. It only rewrites
  values that still name the old folder, so a second run changes nothing else.
- If the new folder already exists without a marker, it does not merge two
  instances: it keeps using the old one and says so in the log.

Hard links cannot cross filesystems. When `~/.local/share/eidos` is a link to
another disk, the move fails cleanly and the instance stays where it is, which
is where the user chose to put it.

Only an old folder that holds an instance (a `mods/` folder or an
`eidos-instance.ini`) is moved or fallen back to. A stray one, such as the
`profiles/` a cancelled setup leaves, stays where it is and is ignored, so a new
global instance of that game is created in the Colony layout.

A folder path that names the old location of a moved instance (an
`EIDOS_INSTANCE` in a Steam launch option, a script) opens the moved instance:
`Instance::portable` redirects it through `eidos_paths::moved_global_instance`.

### Removing the old directories

Nothing deletes them in this release. Once the move has been out long enough to
be trusted, a later release can remove them; until then a user can delete them
by hand once satisfied.
