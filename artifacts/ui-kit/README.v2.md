# trdr terminal-native UI system v2

This package is the framework-neutral source extracted from the approved Phase 1/2 product flow and terminal-native UX kit.

## Files

- `tokens.v2.json` — canonical, DTCG-inspired token source.
- `tokens.v2.css` — resolved CSS custom properties for prototypes and web clients.
- `components.v2.css` — reusable component classes; no screen-specific selectors.
- `contracts.v2.json` — component anatomy, variants, states, events, accessibility, and composition contracts.
- `gallery.v2.html` — visual coverage and composition proof for the current supported surfaces.

## Supported production surface

- Desktop shell at 1440×900 with 196px navigation, fluid structured work area, and 420px persistent raw terminal rail.
- Today: account summary, holdings, disclosures and validation events.
- Lab: normalized strategy rules, data coverage, backtest results, and verdict.
- Strategies: paper-validation list and strategy detail.
- Agent: raw xterm/PTY wrapper, process states, focus, and persistent scrollback.
- Bridge: explicit `trdr ui` navigation and native application approval for durable changes.

## Trust boundary

- Read-only inspection, deterministic backtests, and UI navigation may proceed without an approval dialog.
- Durable local mutations such as strategy registration or deletion pause the CLI and require a native application dialog.
- Approval, rejection, expiration, and errors return to the same CLI session.
- The application styles the terminal wrapper and base ANSI palette. It does not parse output into cards, rewrite layout, or override explicit true-color sequences.

## Explicit exclusions

- Live order entry and broker-side mutations.
- Mobile, responsive collapse, dockable panes, terminal tabs, and plugin UI.
- React, SwiftUI, or another framework binding before the application stack is selected.
- Styling theoretical future screens that are not executed by Phase 1/2.

## Naming rules

- Tokens are semantic after the primitive palette: use `--trdr-color-focus`, not a raw lime value.
- Components use the `trdr-` prefix and describe a reusable role: `trdr-panel`, not `today-card`.
- `data-*` and ARIA attributes carry state using the exact contract value: `data-process="approval-pending"`, `aria-current="page"`, `aria-invalid="true"`.
- Domain recipes compose primitives; they do not introduce a second token system.

## Definition of sufficient coverage

The system is sufficient when Today, Lab draft, Lab result, Strategies, Strategy detail, raw terminal, CLI-to-UI navigation, and approval dialog can be composed from these tokens and components without adding screen-local visual rules. New product behavior adds contracts only when it enters the supported production path.
