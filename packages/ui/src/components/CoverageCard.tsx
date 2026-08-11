import { cssVariables } from "../contracts";

export type CoverageCardState = "loading" | "ready" | "incomplete" | "error";

export type CoverageCardProps =
{
    /** Where the data came from. `KIS 일봉`, `user.krx`. */
    source: string;
    /** The length of the window in days, shown beside the source. */
    days: number;
    /** Days present, and days the window asks for. */
    covered: number;
    total: number;
    /**
     * The coverage in words — `1,190 / 1,258일 · 94.6%`, or what is missing. The
     * percentage has to exist as text, because the bar underneath is
     * supplementary and a screen reader is given none of it.
     */
    summary: string;
    state?: CoverageCardState;
    onInspectMissing?: () => void;
    inspectLabel?: string;
};

/** The share of the window that is present, clamped, as a whole percent. */
function percentage(covered: number, total: number): number
{
    if (total <= 0)
    {
        return 0;
    }

    return Math.max(0, Math.min(100, Math.round((covered / total) * 100)));
}

/**
 * How much of the data a backtest would need is actually here.
 *
 * The numbers are the value and the track is the picture of them. That order is
 * the contract's rule: a progress bar is a comparison someone can take in at a
 * glance and not a figure they can act on, and a run refused for missing days
 * has to say which days.
 */
export function CoverageCard(props: CoverageCardProps)
{
    const { source, days, covered, total, summary, state = "ready" } = props;
    const { onInspectMissing, inspectLabel } = props;
    const percent = percentage(covered, total);

    return (
        <section className="trdr-panel" data-state={state}>
            <div className="trdr-metric">
                <label>{`${source} · ${days}D`}</label>
                <strong className="trdr-num">{`${covered} / ${total}`}</strong>
                <small>{summary}</small>

                <div
                    className="trdr-progress-track"
                    style={cssVariables({ "--trdr-progress": `${percent}%` })}
                    role="progressbar"
                    aria-label={source}
                    aria-valuemin={0}
                    aria-valuemax={total}
                    aria-valuenow={covered}
                    aria-valuetext={summary}
                >
                    <i />
                </div>

                {onInspectMissing === undefined || inspectLabel === undefined ? null : (
                    <button className="trdr-inline-action" type="button" onClick={onInspectMissing}>
                        {inspectLabel}
                    </button>
                )}
            </div>
        </section>
    );
}
