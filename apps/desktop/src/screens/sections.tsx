import { InlineFeedback } from "@trdr/ui";
import type {
    CoverageCardState,
    DataTableState,
    EventListState,
    MetricState,
    SignalLogState
} from "@trdr/ui";

import type { MarketTone, SectionState } from "../bindings";
import { SECTION_STATE } from "../copy/ko";

/**
 * A section's state, in each component's own vocabulary.
 *
 * `SectionState` is per section rather than per screen because a screen is
 * rarely in one state — Today's holdings can be stale while its events are
 * ready — and the components it feeds do not all have the same four words for
 * it. Mapping in one place is what keeps the two rules below from being decided
 * again per screen.
 *
 * **`stale` still shows its data.** Every mapping below sends `stale` to a state
 * that renders rows, because a stale section is one that has its numbers and is
 * admitting they are old. Hiding them would turn "this is from an hour ago" into
 * "there is nothing here". `SectionNote` is what says so beside them.
 *
 * **`empty` is not `error`.** Nothing to show gets an `EmptyState` naming the
 * next step; a section that could not be built gets an alert. A screen that
 * collapsed them would tell someone who has not started yet that something
 * broke.
 */

/** `DataTable` has all four states under the same names. */
export function tableState(state: SectionState): DataTableState
{
    return state;
}

/** `EventList` has no `stale`, so a stale list renders its rows and says so. */
export function eventState(state: SectionState): EventListState
{
    return state === "stale" ? "ready" : state;
}

/** `SignalLog`, for the same reason. */
export function signalState(state: SectionState): SignalLogState
{
    return state === "stale" ? "ready" : state;
}

/** A metric is a number or it is a dash; `stale` is still a number. */
export function metricState(state: SectionState): MetricState
{
    return state === "ready" || state === "stale" ? "ready" : "unavailable";
}

/** `CoverageCard`'s `incomplete` is what a section calls `stale`. */
export function coverageState(state: SectionState): CoverageCardState
{
    if (state === "stale")
    {
        return "incomplete";
    }

    return state === "error" ? "error" : "ready";
}

/**
 * The class a market tone puts on a value.
 *
 * The tone is a field on the model — up is red and down is blue, which is the
 * Korean convention — so no screen derives a colour from the sign of a number.
 * A holding that gained on a day the account lost is exactly the case a sign
 * would get wrong.
 */
export function toneClass(tone: MarketTone): string | undefined
{
    if (tone === "up")
    {
        return "trdr-market-up";
    }

    return tone === "down" ? "trdr-market-down" : undefined;
}

/** Said beside a section that is old or that failed, and nothing otherwise. */
export function SectionNote(props: { readonly state: SectionState })
{
    const { state } = props;

    if (state === "stale")
    {
        return <InlineFeedback tone="warning" message={SECTION_STATE.stale} />;
    }

    if (state === "error")
    {
        return <InlineFeedback tone="error" message={SECTION_STATE.error} />;
    }

    return null;
}
