# M1 risk spikes

Throwaway code. This whole directory is deleted when M1 closes.

Decision `01kzp3xj1p2hmby38fp27xx505`: spikes are built outside the production packages and
deleted when M1 closes; only their recorded answers carry into M2. M1 exists to be able to
reject the Tauri shell, and spike code shaped as the M2 scaffold creates sunk cost against
that rejection.

So nothing here may be imported by `apps/`, `crates/` or `packages/`, and nothing here counts
as the first scaffold. `docs/FOUNDATION_DESIGN.md` section 14 defines what that scaffold
contains, and M2 builds it.

## What each spike has to answer

| Spike | Question it closes | State |
|---|---|---|
| `pty/` | Does a raw terminal survive route changes, resize and an app restart inside Tauri? | **Deleted.** Its answer is production code: `crates/trdr-runtime/src/{env,pty,scrollback}.rs`. |
| `cli-bridge/` | Does a CLI approval request round-trip back to the same waiting CLI process? | Still here. |
| `keychain-kis/` | Can a credential reach an authenticated request without leaking into any artifact? | Still here. |

## Why two of the three are still here

`w-y2b0q` deletes this directory, and it is blocked for a reason it states
itself: the working reference stays until the code that has to reproduce it is
written. That has now happened for the terminal, so `pty/` is gone.

It has not happened for the other two. `crates/trdr-runtime/src/keychain.rs`
and `collectors.rs` are twelve-line modules whose text is "Nothing is
implemented yet", and no AppKit call exists anywhere in `crates/` or `apps/`.
The three answers that live only here are:

- the `Secret` type and how a credential reaches an authenticated request
  without entering a log, an error, or a serialised value (`keychain-kis/`);
- how the Rust host reaches AppKit for a native sheet on the main thread, and
  the borrow rule around a reentrant call (decision `01kzp3xqm6w9z1hr759893ksp6`,
  `cli-bridge/src-tauri/src/sheet.rs`);
- the approval round trip back to the waiting CLI process
  (`cli-bridge/src-tauri/src/approval.rs`).

Milestone M3 reproduces the first and M6 the rest. Each deletion follows the
code that replaces it, in the same change, so that no session has to trust that
an answer was carried across.

## What carries into M2

Answers, not code:

- how the app finds a user's `claude` or `codex` executable when it does not inherit the shell's `PATH`
- how the Rust host reaches AppKit for a native sheet (decision `01kzp3xqm6w9z1hr759893ksp6`)
- the socket protocol shape that survived the failure cases

Each spike writes its answer into the work unit's report. If a spike breaks the trust boundary
or cannot be built reliably on Tauri, M1 stops and the shell choice is revisited.
