# Trusted structured extensions (protocol 1)

Install manifests deliberately into Eidos's user extension directory; archives never
register extensions. `installer-copy.toml` is a complete Python standard-library
example. Keep its `inspect-file.py` beside the manifest. It recognizes only ZIPs
whose extracted source contains `eidos-example.txt`, asks a choice, and returns a
checked file-copy plan. Ordinary ZIPs decline.

An installer manifest uses `kind = "installer"`, `protocol = 1`, filename
`extensions`, optional literal `markers`, and an integer `priority`. Higher
priority runs first; the id breaks ties. Installed custom helpers run before
native layout detection, except a FOMOD remains native unless a helper explicitly
sets priority above 90. Only `declined` automatically tries another candidate.
`manual`, `cancelled`, failure and malformed replies stop the custom path. Native
fallback after `manual` requires a deliberate frontend choice. A recorded required
helper that disappears, changes, or declines is refused without native fallback.

The helper receives a JSON request filename through `{request}`. It must write
exactly one JSON reply to stdout and diagnostics to stderr. Both streams are
limited to 1 MiB and each invocation has a 30-second deadline with cancellation.
`input` is the original archive; `source` is its owned extracted tree; `workspace`
is separate temporary helper scratch space. The source and archive must remain
unchanged. The installer result can select existing source files; it cannot
publish generated scratch files, mutate the profile through its result, or
register archives/plugins. Helpers are trusted programs with the user's filesystem
access. Checked replies and process lifetime limits are not an OS sandbox.

A handled result has this shape:

```json
{"protocol":1,"request_id":"COPY_FROM_REQUEST","outcome":{"status":"handled","result":{"kind":"install","files":[{"source":"eidos-example.txt","destination":"docs/example.txt"}],"warnings":["An optional step was not performed."]}}}
```

Other outcomes are `{"status":"declined"}`, `{"status":"manual","reason":"..."}`,
`{"status":"cancelled"}` or `{"status":"failed","message":"..."}`. A choice is:

```json
{"status":"prompt","id":"readme","title":"Install the readme?","options":["Install","Cancel"],"multiple":false}
```

The host repeats invocation with `answers` mapping the prompt id to zero-based
option indexes. A single-choice answer must contain exactly one index; a multiple
choice can be empty. Indexes must be distinct and in range. A helper must not
repeat an already answered prompt. The host binds saved answers to the complete
original prompt, including title, options and multiplicity, and replays from the
first prompt; stable ids alone do not permit changed options to reuse old answers.
There are at most 64 recorded prompts and 64 options in a prompt.

Source inventory is limited to 65,536 entries, depth 128 and 16 GiB of regular
file data. Sources cannot contain symlinks or special files. A result selects at
most 65,536 files and 16 GiB total; at most 64 warnings of 4,096 bytes each are
accepted. Paths use checked relative components with forward or backward slashes.
Traversal, drive/absolute paths, empty components, trailing spaces/dots, control
characters, Windows device names, wildcards and reserved Eidos metadata destinations
are rejected. Device checks include `COM¹` and `LPT²` as specified by
[Microsoft's naming rules](https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file).
Destination names must be unique without ASCII case sensitivity and cannot overlap
as file/directory paths. Each selected source is checked again and hashed while
copying into private staging. Failed validation or recipe callbacks preserve the
old mod. Custom publication supports fresh installs, staged replacement/backup
and rename; live Merge is refused.

The installed mod stores `.eidos-custom-installer.json`, capped at 16 MiB. Its
versioned receipt binds archive bytes, extracted paths/content, supplied context
values, selected helper id/version/configuration, manifest, resolved executable,
and concrete existing file arguments (such as the Python script). These identity
checks run before the first invocation during recorded replay. The accepted file
plan and warnings are also compared on replay and before publication. Imported
modules, runtime libraries, environment and arbitrary external files a helper
reads are trusted external dependencies; this finite contract does not recursively
pin them. Authors must bump their declared version when those change behavior.

## CLI recorded choices

```sh
eidos install oblivion archive.zip "Example" --save-answers choices.json
```

At a prompt, no mod is installed. The saved envelope is tagged `kind: "custom"`
and includes `receipt` and `prompt`. Add an answer using that exact prompt:

```python
import json
from pathlib import Path
path = Path("choices.json")
record = json.loads(path.read_text())
record["receipt"]["answers"].append({"prompt": record["prompt"], "selected": [0]})
record["prompt"] = None
path.write_text(json.dumps(record, indent=2))
```

```sh
eidos install oblivion archive.zip "Example" --answers choices.json --save-answers choices.json
```

Repeat if another prompt is requested. Completed receipts preserve the selected
plan. To change a completed choice, reopen the installer for a fresh review.
`--answers` and `--save-answers` consume their following filename, never a mod name;
malformed, unknown, duplicate and contradictory CLI options fail explicitly.
Answers files must not replace the source archive. OMOD uses the same outer file
options with `kind: "omod"`; its recorded answers use the native typed OBMM prompt
and answer enum (including text, yes/no and acknowledgements), without flattening
those into custom choice indexes. Requested OMOD profile effects and incomplete
unsupported effects require separate explicit CLI approval flags
`--apply-profile-effects` and `--allow-incomplete`.


## Collection replay and context

Collection members use the same helper and scripted OMOD backends, with recipe
patches and exclusions applied inside staging before publication. A missing answer
stops the collection with its reserved folder intact. The checked local
`.eidos-collection-installer.json` envelope records the collection/member owner,
original archive path, typed receipt and current prompt. Answers can be edited
through the GUI reservation action, or in that file before resuming the collection.
Do not replace receipt identity fields. OMOD review records separate
`apply_profile_effects` and `allow_incomplete` booleans. Unsupported or unapproved
effects remain explicit; a collection never chooses every option automatically.

Hash recipes keep native clone behavior. When a custom or scripted installer is
selected, its approved staged paths and bytes must satisfy the exact hash recipe;
a conflict pauses instead of discarding either selection. Bundle trees stay on the
native recipe path. Only opaque `.archive` cache names use ZIP, 7z or RAR signatures
for helper extension matching; helper requests retain the real source pathname.

Application callers capture current instance/profile controls and active layer
state before invoking custom installers. The context snapshot allows 500,000
filesystem entries, depth 128, 1,024 control files, 8 MiB per control file and
64 MiB total control contents. Active layer source metadata identifies files by
path, inode, size, mtime and ctime; profile controls are content hashed. Effective
mod order/activation is compared independently of modlist serialization, so saving
an already-reconciled inactive reservation does not invalidate its prompt. The
backend recaptures this snapshot before and after helper evaluation and before
publication. Hidden lower-layer changes conservatively require a new review.
Helpers are prefiltered by kind/game/extension before paying for this snapshot;
native OMOD uses its own selected-profile snapshot.

Manifest, executable and concrete script-file argument hashes share a 64 MiB total
limit, with at most 1,024 arguments. Script imports, runtime libraries and environment
remain trusted external dependencies that require an author version bump when
behavior changes. These checks are not an operating-system sandbox.

Approved OMOD profile writes that could not finish retain their backend recovery
receipt. Retry them against the original active profile with:

```sh
eidos install <game-id-or-instance-path> --retry-omod-effects "Installed Mod"
```

The collection engine retries those approved effects before accepting a recovered
published member. Linked, corrupt, oversized, foreign-owner or stale receipts fail
explicitly; missing helpers do not silently redirect to native defaults.
