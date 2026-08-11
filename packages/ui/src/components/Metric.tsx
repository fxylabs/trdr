import type { MarketValueTone } from "../contracts";
import { classes, marketToneClass } from "../contracts";

export type MetricState = "ready" | "loading" | "unavailable";

export type MetricProps =
{
    label: string;
    /**
     * Formatted, unit included — `3,492,000원`, `+8.4%`. The sign is part of the
     * string because the contract asks gain and loss to be present in the text,
     * where a colour cannot be the only thing carrying it.
     */
    value: string;
    /** The number this one is being read against. `학습 +15.7%`. */
    comparison?: string;
    marketTone?: MarketValueTone;
    state?: MetricState;
};

/**
 * One number, and what it is.
 *
 * The tone goes on the value and nowhere else — never the label, never the
 * comparison, never a surface behind them. Colour here means the market moved,
 * so anything wearing it is claiming to be a market number.
 *
 * `Metric` is not a card. In the gallery each one sits inside a panel, and that
 * panel is the screen's decision, not this component's.
 */
export function Metric(props: MetricProps)
{
    const { label, value, comparison, marketTone, state = "ready" } = props;

    return (
        <div className="trdr-metric" data-state={state}>
            <label>{label}</label>

            {state === "loading" ? (
                <strong>
                    <i className="trdr-skeleton trdr-skeleton-line" />
                </strong>
            ) : (
                <strong className={classes("trdr-num", marketToneClass(marketTone))}>
                    {state === "unavailable" ? "—" : value}
                </strong>
            )}

            {comparison === undefined ? null : <small>{comparison}</small>}
        </div>
    );
}
