import { cssVariables } from "../contracts";

export type DayProgressState = "running" | "completed";

export type DayProgressProps =
{
    /** Days observed so far, today included. */
    elapsed: number;
    /** Days the validation window is. Twenty, for a standard paper run. */
    total: number;
    /** Which segment is today, 1-based. Omitted on a completed run. */
    today?: number;
    state?: DayProgressState;
    /** `20일 중 8일 관측`. The bar's value, in words, for anyone not looking. */
    label: string;
};

/** Done, today, or not yet — the segment's state, or nothing for the future. */
function segmentState(day: number, elapsed: number, today: number | undefined): string | undefined
{
    if (today !== undefined && day === today)
    {
        return "today";
    }

    if (today !== undefined ? day < today : day <= elapsed)
    {
        return "done";
    }

    return undefined;
}

/**
 * A validation window, one segment per day.
 *
 * Elapsed days are graphite and today is lime, which is the accent doing the one
 * job it has in this system: pointing at where you are. The days ahead are
 * neither — an empty track is how long there is left to wait, and waiting is
 * the point of paper validation.
 */
export function DayProgress(props: DayProgressProps)
{
    const { elapsed, total, today, state = "running", label } = props;

    return (
        <div
            className="trdr-daybar"
            data-state={state}
            style={cssVariables({ "--trdr-day-count": total })}
            role="progressbar"
            aria-label={label}
            aria-valuemin={0}
            aria-valuemax={total}
            aria-valuenow={elapsed}
        >
            {Array.from({ length: total }, (_unused, index) => index + 1).map((day) => (
                <i key={day} data-state={segmentState(day, elapsed, today)} />
            ))}
        </div>
    );
}
