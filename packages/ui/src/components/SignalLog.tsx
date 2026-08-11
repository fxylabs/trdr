import type { ReactNode } from "react";

export type SignalLogState = "loading" | "ready" | "empty" | "error";

export type SignalContent =
{
    title: ReactNode;
    /**
     * Which registered rule matched, and why this is the consequence of it. A
     * signal with no rule behind it is the thing paper validation exists to
     * catch, so the contract makes the explanation part of the record.
     */
    explanation: ReactNode;
    /** ISO 8601, for the `datetime` attribute. */
    timestamp?: string;
    /** The same moment, as a person reads it. `14:31`. */
    timeLabel?: string;
};

export type SignalLogProps<Signal> =
{
    label: string;
    signals: readonly Signal[];
    signalId: (signal: Signal) => string;
    signal: (signal: Signal) => SignalContent;
    state?: SignalLogState;
    empty?: ReactNode;
    onInspect?: (signalId: string) => void;
};

/**
 * Every signal a registered strategy produced, newest last, with its reason.
 *
 * Chronological list semantics, and a machine-readable timestamp wherever the
 * caller has one — a log whose order is only visual is a log nobody can audit.
 */
export function SignalLog<Signal>(props: SignalLogProps<Signal>)
{
    const { label, signals, signalId, signal, state = "ready", empty, onInspect } = props;

    return (
        <div
            className="trdr-signal-list"
            data-state={state}
            role={state === "empty" ? undefined : "list"}
            aria-label={label}
        >
            {state === "loading"
                ? [0, 1, 2].map((row) => (
                      <div key={row} className="trdr-signal" role="listitem">
                          <i className="trdr-skeleton trdr-skeleton-line" />
                      </div>
                  ))
                : null}

            {state === "empty" ? empty : null}

            {state === "loading" || state === "empty"
                ? null
                : signals.map((item) => (
                      <Signal
                          key={signalId(item)}
                          content={signal(item)}
                          {...(onInspect === undefined ? {} : { onActivate: () => onInspect(signalId(item)) })}
                      />
                  ))}
        </div>
    );
}

function Signal(props: { content: SignalContent; onActivate?: () => void })
{
    const { content, onActivate } = props;
    const { title, explanation, timestamp, timeLabel } = content;

    return (
        <div className="trdr-signal" role="listitem">
            <b>
                {onActivate === undefined ? (
                    title
                ) : (
                    <button className="trdr-inline-action" type="button" onClick={onActivate}>
                        {title}
                    </button>
                )}
            </b>

            <span>
                {timeLabel === undefined ? null : (
                    <>
                        <time {...(timestamp === undefined ? {} : { dateTime: timestamp })}>{timeLabel}</time>
                        {" · "}
                    </>
                )}
                {explanation}
            </span>
        </div>
    );
}
