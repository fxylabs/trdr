/**
 * The state models of `design/ui-kit/contracts.v2.json`, in TypeScript.
 *
 * These are the exact strings a component writes into `data-state` and its
 * neighbours — `stale`, `approval-pending`, `connected-read-only` — and the
 * reason they are typed rather than left to each component is that a hyphen
 * dropped somewhere is a state that silently styles as nothing. `contracts.test.ts`
 * reads the JSON and fails if any list here has drifted from it.
 *
 * The scope class belongs with them: `components.v2.css` hangs the reset, the
 * base type and — the part that is easy to lose — the `:focus-visible` outline
 * off `.trdr-scope`, so anything mounted outside `AppShell` (a portal, a dialog,
 * a test) has to carry it or it renders with no visible focus at all.
 */

import type { CSSProperties } from "react";

/** The class every kit component expects above it. `AppShell` carries it. */
export const TRDR_SCOPE_CLASS = "trdr-scope";

/** Interaction states. All but `disabled` are CSS pseudo-classes, not props. */
export const CONTROL_STATES = ["rest", "hover", "focus-visible", "pressed", "disabled"] as const;
export type ControlState = (typeof CONTROL_STATES)[number];

/** What a surface that is waiting on data can be. */
export const ASYNC_DATA_STATES = ["idle", "loading", "ready", "empty", "stale", "error"] as const;
export type AsyncDataState = (typeof ASYNC_DATA_STATES)[number];

/** The broker link. Read-only is a state, not a footnote. */
export const BROKER_CONNECTION_STATES = [
    "connected-read-only",
    "syncing",
    "stale",
    "disconnected",
    "error"
] as const;
export type BrokerConnectionState = (typeof BROKER_CONNECTION_STATES)[number];

/**
 * The agent process. No component here renders one — the terminal is Track B's —
 * but the strings are the contract's, and both tracks read them from one place.
 */
export const TERMINAL_PROCESS_STATES = [
    "starting",
    "ready",
    "running",
    "needs-input",
    "approval-pending",
    "exited",
    "reconnecting"
] as const;
export type TerminalProcessState = (typeof TERMINAL_PROCESS_STATES)[number];

/** The approval round trip. M6 owns the dialog; the states are declared here. */
export const APPROVAL_STATES = ["requested", "approved", "rejected", "expired", "failed"] as const;
export type ApprovalState = (typeof APPROVAL_STATES)[number];

/** Where a strategy is in its paper validation. */
export const PAPER_VALIDATION_STATES = [
    "draft",
    "ready",
    "running",
    "completed",
    "extended",
    "discarded"
] as const;
export type PaperValidationState = (typeof PAPER_VALIDATION_STATES)[number];

/**
 * Market direction.
 *
 * `up` is red and `down` is blue — the Korean market convention, resolved in
 * `tokens.v2.css` and not to be corrected on sight. A tone applies to a value
 * and never to the label beside it.
 */
export const MARKET_VALUE_TONES = ["up", "down", "flat"] as const;
export type MarketValueTone = (typeof MARKET_VALUE_TONES)[number];

/** The class a market tone puts on a value, or nothing when it is flat. */
export function marketToneClass(tone: MarketValueTone | undefined): string | undefined
{
    if (tone === "up")
    {
        return "trdr-market-up";
    }

    if (tone === "down")
    {
        return "trdr-market-down";
    }

    return undefined;
}

/**
 * CSS custom properties as a style object.
 *
 * Two of the kit's components are driven by one — `--trdr-day-count` sets the
 * number of segments in the day bar, `--trdr-progress` the width of the fill —
 * and a value that comes from data is a value that has to arrive inline. This is
 * the only place any component here writes to `style`, and it never writes a
 * colour, a size or a spacing: those are the stylesheet's.
 */
export function cssVariables(values: Readonly<Record<string, string | number>>): CSSProperties
{
    return values as CSSProperties;
}

/** `["a", undefined, "b"]` to `"a b"`, so a component never emits `"a undefined"`. */
export function classes(...names: readonly (string | false | undefined)[]): string
{
    return names.filter((name): name is string => typeof name === "string" && name.length > 0).join(" ");
}
