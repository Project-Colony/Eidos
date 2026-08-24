# Extensions

An extension adds an entry to Eidos without being part of Eidos. It is a TOML
manifest naming a program, plus, at most, that program.

Manifests live in `~/.config/Colony/Eidos/addons/`, one `.toml` per extension. Open the
folder from **View -> Extensions -> Open folder**, then press **Reload** - no
restart.

## Why nothing is loaded into Eidos

Mod Organizer 2 loads plugins as shared libraries and hosts Python ones through
Qt. Neither transfers. Rust has no stable ABI, so a shared library built against
a different compiler - or a different optimisation flag, or a different feature
set of a shared dependency - is undefined behaviour rather than a version
mismatch. And Eidos's widgets are compile-time generic, so a library could not
build one to hand back even if the ABI were stable.

So an extension is a program Eidos *runs*. It cannot crash the window, cannot
corrupt a mod list, and keeps working across Eidos updates.

## A tool

```toml
id = "wrye-bash"
name = "Wrye Bash"
kind = "tool"
exec = "/opt/wrye/wrye-bash"
args = ["--data", "{data}"]
games = ["skyrimse"]        # omit for every game
description = "Bashed patch builder."
author = "you"
version = "1.0"
```

It appears in **View -> Extensions** with a Run button, and starts detached -
Eidos does not wait for it.

## A check

```toml
id = "esl-count"
name = "ESL budget"
kind = "diagnose"
exec = "/home/me/bin/esl-count.sh"
args = ["{profile_dir}/plugins.txt"]
```

It runs on every refresh and prints one finding per line:

```
level<TAB>title<TAB>detail
```

where `level` is `problem`, `advice` or `ok`. The detail is optional. Anything
that does not begin with a known level is ignored, so progress output and stray
warnings cannot raise a row that looks like one of Eidos's own checks. Findings
appear in the **Health** tab, prefixed with the extension's name.

A check gets three seconds. One that overruns is stopped and reported as a
problem against itself - it runs on the same refresh that follows every click,
so a hanging one would freeze the window.

## Placeholders

Both `args` and `workdir` expand these:

| Placeholder     | What it is                                   |
| --------------- | -------------------------------------------- |
| `{instance}`    | the instance root                            |
| `{mods}`        | `<instance>/mods`                            |
| `{downloads}`   | `<instance>/downloads`                       |
| `{overwrite}`   | `<instance>/overwrite`                       |
| `{profile}`     | the active profile's name                    |
| `{profile_dir}` | the active profile's directory               |
| `{game}`        | the game id, e.g. `skyrimse`                 |
| `{game_name}`   | the game's display name                      |
| `{install}`     | the game install directory                   |
| `{data}`        | the game's `Data` directory                  |

An unknown placeholder is left exactly as written rather than blanked, so a
mistake fails visibly instead of turning `--out {typo}` into `--out --next-flag`.
Running a tool whose placeholders cannot all be resolved is refused, and Eidos
says which ones are missing.

## What an extension cannot do

It gets values and runs; it cannot call back into Eidos, change the mod list, or
draw anything in the window. That is deliberate. The things MO2 uses plugins for
that DO need to reach inside - game support, installers, the conflict engine -
are built in here rather than bolted on: a game definition is its own TOML in
`~/.config/Colony/Eidos/games/`, and FOMOD and BAIN installers are native.
