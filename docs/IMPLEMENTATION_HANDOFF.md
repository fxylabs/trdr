# trdr implementation handoff

> Prepared 2026-08-10 for a model switch. No product implementation has begun.
>
> Start with `self context`, then read this file completely. The project convention assigns
> implementation and review to Opus 5 unless the user reassigns it.
>
> **I0 (`w-nfyc5`) has been executed.** Sections 1, 2, 4 and 6 below are marked with what
> changed. The next session starts at section 7, not at I0.

## 1. Current state

- Repository HEAD: `797d396` (`Add reusable terminal-native UI system`).
- The reusable UI system work `w-rmm0a` is done and its committed files are the visual contract.
- The foundation work `w-vpeht` is done. `docs/FOUNDATION_DESIGN.md` is the process, storage,
  ingest, IPC, authority, migration and backup contract.
- The implementation plan is approved by decision `01kznpf5c9gmevwzt1paqkwerd`.
- The initial contribution policy is confirmed by decision `01kznpx0zk33kws99hpt3jq24c`:
  external code contributions and pull requests are not accepted yet; project changes remain
  issue-based.
- I0 contract and OSS-policy reconciliation `w-nfyc5` is done. Root policy files exist, the
  superseded product documents are archived under `docs/archive/`, and `docs/README.md` is
  the active document index.
- No Tauri, React, Cargo workspace, SQLite schema or CLI scaffold exists yet.
- `~/.trdr` does not exist on the current Mac and must not be created by M0 documentation work.

Do not follow the “Remaining work” section in `artifacts/ui-kit/HANDOFF.v2.md`. That file is a
preserved earlier-session handoff from before `w-rmm0a` was completed. Do not delete or edit it.

## 2. Canonical sources, in order

1. `AGENTS.md` and current `self context`
2. `docs/FOUNDATION_DESIGN.md`
3. `docs/IMPLEMENTATION_PLAN.md`
4. `artifacts/ui-kit/README.v2.md`
5. `artifacts/ui-kit/contracts.v2.json`
6. `artifacts/ui-kit/tokens.v2.json`, `tokens.v2.css`, `components.v2.css`

I0 resolved the conflicting documents. `README.md` was rewritten to the confirmed contracts.
`PRD.md`, `MVP_SCOPE.md`, `USER_JOURNEY.md`, `USER_STORIES.md`, `SCREEN_ACTIONS.md` and
`SCREEN_COMPOSITION.md` moved to `docs/archive/` with their bodies unchanged and an archive
banner naming what replaced each. No separate active PRD was written: product requirements are
sections 1 to 4 of `docs/IMPLEMENTATION_PLAN.md`, so that one document stays the only answer.

## 3. Confirmed product and architecture decisions

- Full local desktop product, including UI, is OSS under AGPLv3.
- `trdr` name, logo, official Apple signature, notarized DMG and update identity are reserved.
- Tauri 2 + React/TypeScript/Vite UI + Rust host/core/CLI; no Electron or SwiftUI rewrite.
- Fixed single-window Today/Lab/Strategies product surfaces with a persistent right-side raw PTY.
- Apple Silicon macOS 14+ only for v0; direct notarized DMG, not the App Store.
- No orders, broker mutation, automatic trading, cloud backend or telemetry-dependent feature.
- KIS is the only v0 broker and is read-only.
- Built-in local collectors are KIS, OpenDART and ECOS. KRX is absent from the official path.
- User collectors are out of process and insert only with a `user.*` versioned bundle through
  `trdr data ingest`; no plugin runtime and no direct SQLite writes.
- Product root is `~/.trdr`; default workspace is `~/.trdr/workspaces/default`.
- `com.fxylabs.trdr` is the bundle identifier and Keychain service.
- SQLite 3.51.3+ with WAL and a single-writer lease is the v0 system of record.
- Rust ownership is `trdr-core`, `trdr-runtime`, and `trdr-cli`, composed by the Tauri host.
- AppKit native surfaces are limited to credentials, backup passphrase, registration approval,
  and workspace restore/switch approval.
- First scaffold uses synthetic data only. Live APIs, collectors, backtesting and registration
  are later gated work.
- Migration recovery points use SQLite's backup API. Portable full backup is encrypted and
  restores into a new workspace. No incremental or cloud backup in v0.

Decision IDs are collected in `docs/FOUNDATION_DESIGN.md` section 15 and current `self state`.

## 4. Worktree ownership

At handoff time:

```text
 M AGENTS.md
 M CLAUDE.md
?? artifacts/ui-kit/HANDOFF.v2.md
?? docs/FOUNDATION_DESIGN.md
?? docs/IMPLEMENTATION_PLAN.md
?? docs/IMPLEMENTATION_HANDOFF.md
```

- `AGENTS.md`, `CLAUDE.md`, and `artifacts/ui-kit/HANDOFF.v2.md` are user-owned pre-existing
  changes. Preserve them and do not stage, rewrite, or remove them.
- The three `docs/` files are the uncommitted planning/handoff artifacts produced for this
  implementation transition. I0 should reconcile and commit them intentionally with the current
  contract documents; do not mix user-owned files into that commit.
- Always inspect status before staging. Never use `git add -A` for the I0 commit.

After I0: `AGENTS.md`, `CLAUDE.md` and `artifacts/ui-kit/HANDOFF.v2.md` are still uncommitted
and still user-owned. The I0 commit staged only the documents and policy files it created or
moved. The same rule holds for the next session.

## 5. Contribution and issue policy

- External code contributions and pull requests are not accepted during the initial OSS phase.
- Project changes are issue-based: implementation is tracked against an accepted project issue.
- `CONTRIBUTING.md` must state that external code contributions are currently closed and direct
  bug reports and proposals to issues.
- DCO, CLA, contributor licensing, fork PRs and merge policy are deliberately deferred. Decide
  them before opening external contributions; do not infer relicensing rights from the AGPL release.

This policy supersedes proposal `01kznpf9a2wy1kyjchdpcd09pd` and is recorded by decision
`01kznpx0zk33kws99hpt3jq24c`.

`CONTRIBUTING.md` now carries this policy.

## 6. I0 narrow review dispatch contract (executed)

### Required behavior

- Make root README, PRD, MVP scope and active supporting docs agree with the confirmed fixed-shell,
  Tauri, OSS, KIS/OpenDART/ECOS and bundle-ingest contracts.
- Remove or explicitly archive Electron, dockview, free-pane, official KRX, paid-app, Toss,
  hosted-data and live-order statements from the active v0 path.
- Add `LICENSE` (AGPLv3), `TRADEMARKS.md`, `SECURITY.md`, `PRIVACY.md`, `CONTRIBUTING.md`, and a
  third-party-notice policy. `CONTRIBUTING.md` closes external code contributions for now and
  defines issues as the intake and work-tracking path.
- Reconcile `package.json` from `UNLICENSED`/`private: true` to the approved OSS metadata state.
- Preserve source attribution and historical documents rather than silently rewriting evidence.

### Touched production surfaces

- Root project metadata and policy files
- Active product/implementation documentation
- Package metadata only

### Supported inputs and trust boundary

- The confirmed decisions and canonical sources in sections 2 and 3 above
- Existing committed UI kit v2
- No credentials, account data, market data or external source code

### Explicit exclusions

- No dependency installation
- No Tauri/React/Cargo scaffold
- No `~/.trdr` creation
- No SQLite schema or migration
- No Keychain, PTY, IPC, collector, backtest or UI implementation
- No broad historical-document rewrite beyond resolving active production-path conflicts

### Stop condition

I0 is sufficient when an implementation model can read the active repository documents and get
one answer for shell, platform, license, data sources, trust boundary and first executable slice;
dependency-license review has no AGPL blocker; policy files exist; only intended files are staged;
and `git diff --check` passes. Do not expand I0 into product code or theoretical future hardening.

### What I0 produced

| Surface | Result |
|---|---|
| `README.md` | Rewritten to the fixed Tauri shell, macOS scope, AGPLv3, KIS/OpenDART/ECOS and bundle ingest |
| `LICENSE` | AGPLv3 verbatim from gnu.org, SHA-256 `0d96a4ff68ad6d4b6f1f30f713b18d5184912ba8dd389f86aa7710db079abcb0` |
| `CONTRIBUTING.md` | External code contributions closed; issues are the intake and tracking path; DCO/CLA deferred |
| `SECURITY.md` | Private advisory channel, v0 trust boundary, blocking classes, out-of-scope classes |
| `PRIVACY.md` | What leaves the Mac, what never does, storage locations, telemetry off, backup sensitivity |
| `TRADEMARKS.md` | Reserved name, logo, signing and update identity; fork renaming checklist |
| `THIRD_PARTY_NOTICES.md` | Allowed and blocked license classes, dependency rules, current state: no dependencies exist |
| `package.json` | `license` now `AGPL-3.0-only`; `homepage` and `bugs` added |
| `docs/README.md` | Question-to-document index and the precedence order when two documents disagree |
| `docs/archive/` | Six superseded documents, bodies unchanged, each with an archive banner |

Two judgment calls, both open to reversal:

1. **No active PRD was written.** Product requirements are sections 1 to 4 of
   `docs/IMPLEMENTATION_PLAN.md`. A second document restating them is how the first conflict
   started.
2. **`"private": true` stays in `package.json`.** The repository root is a workspace root, not
   a published npm package, and the flag only blocks `npm publish`. `UNLICENSED` was the actual
   conflict with the OSS decision, and it is gone.

## 7. After I0

Register and execute only the three M1 risk spikes from `docs/IMPLEMENTATION_PLAN.md`:

1. raw Claude/Codex PTY persistence and resize;
2. Keychain canary plus mock KIS request with non-disclosure evidence;
3. Unix-socket CLI bridge with approve/reject/expire round-trip.

If a spike breaks the foundation trust boundary, stop and revisit that path before building the
walking skeleton. Do not skip directly to broad UI implementation.

## 8. Suggested next-session prompt

> Read `self context` and `docs/IMPLEMENTATION_HANDOFF.md`. I0 is done, so start at section 7:
> register and execute only the three M1 risk spikes. Preserve the existing user-owned worktree
> changes in `AGENTS.md`, `CLAUDE.md` and `artifacts/ui-kit/HANDOFF.v2.md`, and do not begin the
> walking skeleton until all three spikes report evidence.
