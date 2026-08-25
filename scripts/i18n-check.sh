#!/usr/bin/env bash
# Whether every translated document still matches the English it was made from.
#
# Each translation carries the BLOB HASH of its source, not a commit sha:
#
#     <!-- eidos-i18n: source=docs/guide/install.md sha=62a0541b -->
#
# The hash is `git hash-object <source>`, computable from the working tree with
# no history at all. That is deliberate. The obvious design - record the last
# commit that touched the source and compare with `git log` - needs history this
# repo's CI does not fetch: `actions/checkout` clones with depth 1 unless told
# otherwise, `git log` on a shallow clone reports one commit, every comparison
# comes out equal, and the check passes green forever while translations rot.
# A freshness check that cannot fail is worse than none: it is a badge asserting
# something nobody verified.
#
# Exit 1 if any translation is stale or malformed. `--fix` restamps instead,
# for when you have just re-translated.
set -uo pipefail
cd "$(dirname "$0")/.."

fix=0
stamp=0
case "${1:-}" in
    --fix) fix=1 ;;
    # Adds a missing header to a translated file. An authoring convenience, kept
    # separate from --fix on purpose: CI must keep FAILING on an unstamped file,
    # because a translation with no stamp is one nothing can ever tell is stale.
    --stamp) stamp=1 ;;
esac

stale=0
missing=0
checked=0

# Translations are `<name>.<lang>.md` beside their English source, so the source
# is the same path with the language segment removed. Sibling suffix rather than
# a mirrored tree because 16 links in docs/ climb out of their own directory -
# 8 of them into crates/ - and a mirror would need every one of those rewritten
# per language, silently breaking at the next restructure.
while IFS= read -r t; do
    checked=$((checked + 1))
    # Strip `find`'s leading `./`: the stamp inside a file names its source the
    # way a human would write it, repo-relative, and a path that does not match
    # the stamp byte for byte makes --fix silently substitute nothing.
    t="${t#./}"
    src="$(printf '%s' "$t" | sed -E 's/\.[a-z]{2}(-[A-Za-z]+)?\.md$/.md/')"
    if [ ! -f "$src" ]; then
        printf 'ORPHAN  %s\n        its English source %s does not exist\n' "$t" "$src"
        missing=$((missing + 1))
        continue
    fi
    want="$(git hash-object "$src")"
    have="$(sed -n 's/.*eidos-i18n: source=[^ ]* sha=\([0-9a-f]*\).*/\1/p' "$t" | head -1)"
    if [ -z "$have" ]; then
        if [ "$stamp" = 1 ]; then
            printf '<!-- eidos-i18n: source=%s sha=%s -->\n\n%s\n' "$src" "$want" "$(cat "$t")" > "$t.tmp"
            mv "$t.tmp" "$t"
            printf 'STAMPED %s\n' "$t"
            continue
        fi
        printf 'NO STAMP %s\n         add: <!-- eidos-i18n: source=%s sha=%s -->\n' "$t" "$src" "$want"
        missing=$((missing + 1))
        continue
    fi
    if [ "$have" != "$want" ]; then
        if [ "$fix" = 1 ]; then
            sed -i "s|eidos-i18n: source=$src sha=$have|eidos-i18n: source=$src sha=$want|" "$t"
            # Confirm the substitution LANDED before saying so. The first version
            # of this script printed RESTAMPED unconditionally, and because a
            # path mismatch made the pattern match nothing, it announced success
            # while changing not one byte - the exact failure this whole check
            # exists to prevent, inside the check itself.
            if [ "$(sed -n 's/.*eidos-i18n: source=[^ ]* sha=\([0-9a-f]*\).*/\1/p' "$t" | head -1)" = "$want" ]; then
                printf 'RESTAMPED %s\n' "$t"
            else
                printf 'COULD NOT RESTAMP %s\n                  its stamp does not name %s\n' "$t" "$src"
                stale=$((stale + 1))
            fi
        else
            n="$(git log --oneline "$src" 2>/dev/null | wc -l)"
            printf 'STALE   %s\n        %s changed since this was translated (stamped %s, now %s)\n' \
                "$t" "$src" "${have:0:8}" "${want:0:8}"
            [ "$n" -gt 1 ] && printf '        history: %s commits on the source\n' "$n"
            stale=$((stale + 1))
        fi
    fi
done < <(find . -name '*.[a-z][a-z].md' -o -name '*.[a-z][a-z]-[A-Za-z]*.md' | grep -v './target/' | sort)

printf '\n%s translation(s) checked' "$checked"
[ "$stale" -gt 0 ] && printf ', %s STALE' "$stale"
[ "$missing" -gt 0 ] && printf ', %s unusable' "$missing"
printf '\n'

if [ "$stale" -gt 0 ] || [ "$missing" -gt 0 ]; then
    cat <<'MSG'

A stale translation is worse than a missing one: it looks authoritative and
gives last month's instructions. Either re-translate it and run this with
--fix, or delete it and let the reader fall back to English.
MSG
    exit 1
fi
exit 0
