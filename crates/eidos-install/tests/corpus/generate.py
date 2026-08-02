"""Derive an anonymised layout corpus from a real Eidos instance.

Only the SHAPE of each tree survives: directory names that carry meaning for the
checker (the Gamebryo vocabulary, `data`, `root`, `fomod`, the BAIN-ignored set)
are kept because they ARE the thing under test; everything else becomes `dN`, and
files keep only their extension. No mod name reaches the repository.
"""
import os, subprocess, sys, collections

KEEP = {
    "fonts", "interface", "menus", "meshes", "music", "scripts", "shaders", "sound",
    "strings", "textures", "trees", "video", "facegen", "materials", "skse", "obse",
    "mwse", "nvse", "fose", "f4se", "distantlod", "asi", "skyproc patchers", "tools",
    "mcm", "icons", "bookart", "distantland", "mits", "splash", "dllplugins",
    "calientetools", "netscriptframework", "shadersfx",
    "data", "root", "fomod", "omod conversion data", "images", "screenshots", "docs",
    "facegendata", "plugins",
    # Unreal / Stellar Blade. These are ENGINE directory names, not mod names, and
    # the checker reads them the same way it reads `meshes`: `sb` is the game's own
    # directory at the install root, `~mods` and `logicmods` are its data folders,
    # and `content/paks` is what tells a root-relative archive apart from one that
    # would be shadowed by the data mount.
    "sb", "engine", "content", "paks", "~mods", "logicmods", "binaries", "win64",
    "ue4ss", "mods",
}

# Where each game's real mod roots and real downloaded archives live on the
# machine this was generated from. A source that is absent is simply skipped, so
# the script still runs on a machine that has only one of these games.
SOURCES = [
    {
        "game": "skyrimse",
        "mods": os.path.expanduser("~/.local/share/eidos/skyrimse/mods"),
        "skip_suffix": "_separator",
        "archives": [os.path.expanduser("~/.local/share/eidos/skyrimse/downloads")],
    },
    {
        # Deployed by Vortex, which is why the mod roots and the archives it never
        # unpacked sit in the same directory.
        "game": "stellarblade",
        "mods": "/mnt/Jeux/SteamLibrary/steamapps/common/StellarBlade/SB/Content/Paks/~mods",
        "skip_suffix": None,
        "archives": ["/mnt/Jeux/SteamLibrary/steamapps/common/StellarBlade/SB/Content/Paks/~mods"],
    },
]

# The checker never looks past the second level of a mod root (`data_looks_valid`
# reads level 1, `bain_subpackages` reads each top-level dir's own contents), and
# never past a wrapper chain in an archive. Anything deeper is bulk, not shape.
MOD_DEPTH = 2
ARCHIVE_DEPTH = 3

# A textures folder with 500 .dds files exercises the checker exactly as hard as
# one with two. Cap siblings so the corpus stays readable and diffable.
MAX_PER_EXT = 2
MAX_DIRS = 12


class Anon:
    """Stable per-parent renaming, so two siblings never collapse into one."""

    def __init__(self):
        self.dirs = {}
        self.files = {}

    def dir(self, parent, name):
        low = name.lower()
        if low in KEEP:
            return low
        if low.startswith("--"):
            return "--d" + str(len(self.dirs))
        key = (parent, low)
        if key not in self.dirs:
            self.dirs[key] = "d%d" % len([k for k in self.dirs if k[0] == parent])
        return self.dirs[key]

    def file(self, parent, name):
        low = name.lower()
        ext = low.rsplit(".", 1)[1] if "." in low else ""
        key = (parent, low)
        if key not in self.files:
            n = len([k for k in self.files if k[0] == parent])
            self.files[key] = ("f%d.%s" % (n, ext)) if ext else ("f%d" % n)
        return self.files[key]


def shape_dir(root, max_depth):
    """Anonymised `/`-joined paths under `root`, directories suffixed with `/`."""
    a = Anon()
    out = []

    def walk(abs_dir, rel, depth):
        if depth > max_depth:
            return
        try:
            entries = sorted(os.scandir(abs_dir), key=lambda e: e.name.lower())
        except OSError:
            return
        per_ext, dirs = collections.Counter(), 0
        for e in entries:
            if e.name == "meta.ini" and depth == 1:
                continue
            if e.is_dir(follow_symlinks=False):
                if dirs >= MAX_DIRS:
                    continue
                dirs += 1
                name = a.dir(rel, e.name)
                out.append(rel + name + "/")
                walk(e.path, rel + name + "/", depth + 1)
            else:
                ext = e.name.lower().rsplit(".", 1)[1] if "." in e.name else ""
                per_ext[ext] += 1
                if per_ext[ext] > MAX_PER_EXT:
                    continue
                out.append(rel + a.file(rel, e.name))

    walk(root, "", 1)
    return out


def shape_archive(path, max_depth):
    """Same, from a 7z listing. Never extracts."""
    try:
        raw = subprocess.run(
            ["7z", "l", "-ba", "-slt", path], capture_output=True, text=True, timeout=120
        ).stdout
    except Exception:
        return []
    items, cur = [], {}
    for line in raw.splitlines():
        if line.startswith("Path = "):
            if cur:
                items.append(cur)
            cur = {"path": line[7:]}
        elif line.startswith("Attributes = ") and cur:
            cur["dir"] = "D" in line[13:].split()[0]
    if cur:
        items.append(cur)

    a = Anon()
    out = []
    per_ext, dirs = collections.Counter(), collections.Counter()
    for it in sorted(items, key=lambda i: i["path"].lower()):
        parts = [p for p in it["path"].replace("\\", "/").split("/") if p]
        if not parts or len(parts) > max_depth:
            continue
        parent = "/".join(parts[:-1]).lower()
        leaf = parts[-1]
        if it.get("dir", False):
            dirs[parent] += 1
            if dirs[parent] > MAX_DIRS:
                continue
        else:
            ext = leaf.lower().rsplit(".", 1)[1] if "." in leaf else ""
            per_ext[(parent, ext)] += 1
            if per_ext[(parent, ext)] > MAX_PER_EXT:
                continue
        rel = ""
        for i, p in enumerate(parts):
            last = i == len(parts) - 1
            if last and not it.get("dir", False):
                rel += a.file(rel, p)
            else:
                rel += a.dir(rel, p) + "/"
        out.append(rel)
    return sorted(set(out))


def main():
    cases = []
    seen = set()

    for src in SOURCES:
        game, mods = src["game"], src["mods"]
        skip = src["skip_suffix"]
        if os.path.isdir(mods):
            for name in sorted(os.listdir(mods)):
                p = os.path.join(mods, name)
                if not os.path.isdir(p) or (skip and name.endswith(skip)):
                    continue
                paths = shape_dir(p, MOD_DEPTH)
                if not paths:
                    continue
                key = (game, "modroot") + tuple(paths)
                if key in seen:
                    continue
                seen.add(key)
                cases.append(("modroot", game, paths))

        for adir in src["archives"]:
            if not os.path.isdir(adir):
                continue
            for name in sorted(os.listdir(adir)):
                if not name.lower().endswith((".7z", ".zip", ".rar")):
                    continue
                paths = shape_archive(os.path.join(adir, name), ARCHIVE_DEPTH)
                if not paths:
                    continue
                key = (game, "archive") + tuple(paths)
                if key in seen:
                    continue
                seen.add(key)
                cases.append(("archive", game, paths))

    out = [
        "# Anonymised layout corpus, derived from a real Skyrim SE instance.",
        "# Generated once; it is the INPUT half of the characterisation test.",
        "# Meaningful directory names are kept because they are what the checker",
        "# reads; every other name is dN and files keep only their extension.",
        "#",
        "# One case per block: a `> <kind> <id> game=<id>` header, then its paths.",
        "",
    ]
    n = collections.Counter()
    for kind, game, paths in cases:
        n[(game, kind)] += 1
        out.append("> %s %03d game=%s" % (kind, n[(game, kind)], game))
        out.extend(paths)
        out.append("")
    sys.stdout.write("\n".join(out))
    for k in sorted(n):
        sys.stderr.write("%s %s=%d\n" % (k[0], k[1], n[k]))


main()
