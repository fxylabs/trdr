#!/usr/bin/env bash
#
# Holds `trdr-core` to its dependency boundary.
#
# `crates/trdr-core` is the domain crate: identity, the error envelope, and the
# two IPC contracts. It is pure by design, and its Cargo.toml says so in a
# comment — no Tauri, no tokio, no rusqlite, no reqwest, nothing that reaches
# the filesystem, the network, the clock, or the OS. A comment is not a gate,
# though, and the crate that everything else depends on is exactly the one where
# an accidental I/O dependency would be least visible in review.
#
# So the boundary is checked rather than described. The five crates below are
# the whole of `trdr-core`'s normal dependency tree; their own transitive
# dependencies come along with them and are not the subject here. Dev- and
# build-dependencies are deliberately out of scope: they never ship, which is
# why `--edges normal` is the right edge set.
#
# Run it from anywhere in the repository.

set -euo pipefail

ALLOWED=$(
    cat <<'CRATES'
serde
serde_json
specta
thiserror
ulid
CRATES
)

cd "$(dirname "$0")/.."

# `--prefix none` drops the tree glyphs and leaves one `name version` line per
# crate, starting with trdr-core itself. `--locked` refuses to quietly update
# Cargo.lock, so a lockfile that does not match the manifests fails here too.
#
# trdr-core is dropped inside awk rather than by a following `grep -v`: with
# `pipefail` set, a grep that matches nothing exits 1, so a core that had lost
# every dependency would kill the script instead of reporting what went missing.
actual=$(
    cargo tree --package trdr-core --edges normal --depth 1 --prefix none --locked |
        awk 'NF && $1 != "trdr-core" { print $1 }' |
        sort -u
)

expected=$(printf '%s\n' "$ALLOWED" | sort -u)

if [ "$actual" = "$expected" ]; then
    echo "trdr-core depends on exactly the five crates it is allowed to depend on."
    exit 0
fi

added=$(comm -13 <(printf '%s\n' "$expected") <(printf '%s\n' "$actual"))
removed=$(comm -23 <(printf '%s\n' "$expected") <(printf '%s\n' "$actual"))

echo "trdr-core's dependency boundary moved." >&2
echo >&2

if [ -n "$added" ]; then
    echo "Added, and not on the allowlist:" >&2
    printf '%s\n' "$added" | awk '{ print "  " $0 }' >&2
    echo >&2
    echo "If the new crate performs I/O — filesystem, network, clock, OS, a" >&2
    echo "database, or a UI toolkit — it does not belong in trdr-core. Move" >&2
    echo "the code that needs it to trdr-runtime, which is where effects" >&2
    echo "live, and leave the value types behind in core." >&2
    echo >&2
    echo "If it is genuinely a pure data crate and the boundary should grow," >&2
    echo "that is a design decision, not a build fix. Get it agreed, then add" >&2
    echo "the crate to the ALLOWED list in ci/check-core-deps.sh in the same" >&2
    echo "commit that adds the dependency, so the two are reviewed together." >&2
fi

if [ -n "$removed" ]; then
    [ -n "$added" ] && echo >&2
    echo "On the allowlist, but no longer depended on:" >&2
    printf '%s\n' "$removed" | awk '{ print "  " $0 }' >&2
    echo >&2
    echo "If dropping it was intended, remove it from the ALLOWED list in" >&2
    echo "ci/check-core-deps.sh." >&2
fi

exit 1
