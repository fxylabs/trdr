# Handoff — reusable terminal-native UI system v2

## Work unit

- `w-rmm0a`
- Outcome: extract the approved Phase 1/2 UI into a framework-neutral reusable system and prove the current surfaces need no screen-local visual rules.
- Base commit before this uncommitted work: `3d5bf7e` (`Add terminal-native UI UX kit`).

## Narrow review contract

- Required: reusable tokens, components, state and event contracts for Today, Lab draft/result, Strategies, Strategy detail, raw xterm rail, CLI-to-UI navigation, and native approval.
- Supported inputs: Korean UI copy, tabular financial values, current KIS read-only fixture, normalized strategy/backtest fixture, raw Claude or Codex PTY output.
- Trust boundary: local desktop app, local PTY, local trdr CLI, and the user's configured read-only broker connection.
- Excluded: live orders, broker mutations, mobile, dockable panes, plugin UI, and framework-specific bindings.
- Stop condition: the approved Phase 1/2 surfaces can be composed without new screen-local visual rules and the four gallery frames render cleanly.

## Uncommitted files created

- `design/ui-kit/tokens.v2.json` — canonical DTCG-inspired token source.
- `design/ui-kit/tokens.v2.css` — resolved CSS custom properties.
- `design/ui-kit/components.v2.css` — framework-neutral reusable component classes.
- `design/ui-kit/contracts.v2.json` — 26 component contracts, 7 state models, 7 composition recipes, and 5 CLI protocols.
- `design/ui-kit/README.v2.md` — scope, trust boundary, naming and sufficiency rules.
- `design/ui-kit/gallery.v2.html` — four-frame visual coverage gallery.
- `design/ui-kit/gallery.v2_01-primitives.png`
- `design/ui-kit/gallery.v2_02-data-strategy.png`
- `design/ui-kit/gallery.v2_03-agent.png`
- `design/ui-kit/gallery.v2_04-coverage.png`

## Evidence already obtained

- Both JSON sources parse successfully with Node.
- `contracts.v2.json` reports 26 component contracts.
- `git diff --check` passed after all source files were created.
- All four gallery PNG files were produced by local headless Chrome at the requested 1600×1050 viewport.
- Frame 01, Primitives, was visually inspected and is clean: typography, controls, feedback, focus hierarchy, panel, loading and empty states all render as intended.

## Remaining work

1. Start or resume `w-rmm0a` with `self work start w-rmm0a`.
2. Visually inspect gallery frames 02–04 with `view_image`:
   - data and strategy composition;
   - raw agent rail, six visible process states and ANSI palette;
   - component coverage, CLI authority table and approval dialog.
3. Confirm all four PNG dimensions with `sips -g pixelWidth -g pixelHeight`.
4. Fix only visible or contract-breaking issues. Do not expand into future components.
5. Re-run JSON parse and `git diff --check`.
6. Stage only the v2 files listed above. Existing `AGENTS.md` and `CLAUDE.md` modifications belong to the user and must remain unstaged.
7. Commit with a focused message such as `Add reusable terminal-native UI system`.
8. Report evidence to `w-rmm0a`. Mark done only if the stated stop condition is met.

## Operational note

Parallel headless Chrome renders left long-running sessions and made later shell calls slow. Their four exec sessions were interrupted after the PNGs had been written. If a rerender is needed, run one frame at a time and terminate the headless session after the `bytes written` message. No source file was lost or partially written.
