# Extensions

An extension adds an entry to Eidos without being part of Eidos. It is a TOML
manifest naming a program, plus, at most, that program.

Manifests live in `~/.config/Colony/Eidos/addons/`, one `.toml` per extension.
Open the folder from View -> Extensions -> Open folder, then press Reload - no
restart.

`id`, `kind` and `exec` are required; `name` defaults to the id. `exec` names
the program. A name without a `/` is looked up on `PATH`. When the program is
missing, Eidos skips the extension, and its row in View -> Extensions says why
(`<exec> is not on PATH` or `<exec> is not there`). `games` limits the
extension to those game ids; without it, the extension applies to every game,
and on another game its row says "not for this game".

A manifest Eidos refuses is listed first, with the reason: a file it cannot
read, not valid TOML or a required field missing, an empty `id` or one with a
space, an empty `exec`, an `id` an earlier manifest already took (files are
read in name order), a `kind` Eidos does not know, or an installer, preview or
save-info kind without `protocol = 1` and filename `extensions`.

## Why nothing is loaded into Eidos

Mod Organizer 2 loads plugins as shared libraries and hosts Python ones through
Qt. Neither transfers. Rust has no stable ABI, so a shared library built against
a different compiler - or a different optimisation flag, or a different feature
set of a shared dependency - is undefined behaviour, and nothing reports it as
a version mismatch. And Eidos's widgets are compile-time generic, so a library
could not build one to hand back even if the ABI were stable.

An extension is a program Eidos runs with your account's filesystem access.
Install only helpers you trust. Process isolation and checked replies do not
provide an operating-system sandbox.

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

It appears in View -> Extensions with a Run button, and starts detached -
Eidos does not wait for it, and the status line gives its pid. Its output is
discarded. Without `workdir` it starts in the folder Eidos itself was started
from.

## A check

```toml
id = "esl-count"
name = "ESL budget"
kind = "diagnose"
exec = "/home/me/bin/esl-count.sh"
args = ["{profile_dir}/plugins.txt"]
```

It runs on every refresh, in the instance root unless `workdir` says
otherwise, and prints one finding per line:

```
level<TAB>title<TAB>detail
```

where `level` is `problem`, `advice` or `ok`. The detail is optional; a line
with no title is dropped. Anything that does not begin with a known level is
ignored, so progress output and stray warnings cannot raise a row that looks
like one of Eidos's own checks. Findings appear in the Diagnostics tab as
`<name> - <title>`, after the prefix and Nexus cards and before the other
Health cards.

A check gets three seconds. One that overruns is stopped and reported as a
problem against itself - it runs on the same refresh that follows every click,
so a hanging one would freeze the window. The clock runs until the check has
exited and its output pipes are closed, so a background child that still holds
them counts against the three seconds. A check that exits with an error and
prints nothing is reported the same way, with the first line of its error
output, and so is one that prints more than 1 MiB on either stream. Either way
Eidos does not run it again until you press Reload, and its row says "It will
not be retried until you press Reload." A check reads nothing on its standard
input.

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
| `{install}`     | the game folder the instance runs            |
| `{data}`        | that folder's `Data` directory               |

`{install}` is the store install, or the game copy when the instance runs one
(a frozen `Stock Game` or a list's own game folder; see
[usage.md](usage.md#game-files-the-store-install-or-a-copy)). `{data}` follows
it.

A check also gets `{addon_dir}`, the folder that holds its manifest, and so do
the protocol-1 extensions below. A tool does not.

An unknown placeholder is never blanked, which would turn `--out {typo}` into
`--out --next-flag`. Eidos refuses to run a tool or a check whose placeholders
cannot all be resolved, and says which ones are missing: in the status line for
a tool, in the check's row in the Diagnostics tab for a check.

## What an extension cannot do

An extension cannot call back into Eidos, register itself from an archive, or draw
its own widgets. Structured protocol-1 installers can return checked file plans
and choice prompts; Eidos owns the review, staging and publication. FOMOD and BAIN
remain native installers. Preview and save-info extensions use the same bounded
reply host. See the [protocol contract and runnable examples](examples/extensions/README.md)
for matching priority, recorded CLI answers, limits and replay identity checks.
