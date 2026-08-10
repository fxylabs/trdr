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

| Spike | Question it closes |
|---|---|
| `pty/` | Does a raw terminal survive route changes, resize and an app restart inside Tauri? |
| `cli-bridge/` | Does a CLI approval request round-trip back to the same waiting CLI process? |
| `keychain-kis/` | Can a credential reach an authenticated request without leaking into any artifact? |

## What carries into M2

Answers, not code:

- how the app finds a user's `claude` or `codex` executable when it does not inherit the shell's `PATH`
- how the Rust host reaches AppKit for a native sheet (decision `01kzp3xqm6w9z1hr759893ksp6`)
- the socket protocol shape that survived the failure cases

Each spike writes its answer into the work unit's report. If a spike breaks the trust boundary
or cannot be built reliably on Tauri, M1 stops and the shell choice is revisited.
