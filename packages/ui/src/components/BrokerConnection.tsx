import type { BrokerConnectionState } from "../contracts";

export type BrokerConnectionProps =
{
    /** The broker's name, as the user knows it. `KIS`, `Toss`. */
    broker: string;
    state: BrokerConnectionState;
    /**
     * The state in words. Required, because the contract says the state is in
     * the text and not in the colour, and this package holds no copy of its own.
     */
    stateLabel: string;
    /** Already formatted. `14:31 동기화` — the kit does not format times. */
    lastSync?: string;
    /** `읽기 전용`. What the connection is allowed to do, in words. */
    authority?: string;
    onOpenSettings?: () => void;
    onRetry?: () => void;
    settingsLabel?: string;
    retryLabel?: string;
};

/**
 * The broker link at the foot of the sidebar.
 *
 * The dot is green when the connection is up, and green means exactly that —
 * the system is connected. It never reports whether the account made money;
 * that is `Metric`'s market tone, and it is red and blue.
 */
export function BrokerConnection(props: BrokerConnectionProps)
{
    const { broker, state, stateLabel, lastSync, authority } = props;
    const { onOpenSettings, onRetry, settingsLabel, retryLabel } = props;

    return (
        <div className="trdr-connection" data-state={state}>
            <strong>{broker}</strong>

            <div>
                {stateLabel}
                {authority === undefined ? null : ` · ${authority}`}
            </div>

            {lastSync === undefined ? null : <div className="trdr-num">{lastSync}</div>}

            {onRetry === undefined || retryLabel === undefined ? null : (
                <button className="trdr-inline-action" type="button" onClick={onRetry}>
                    {retryLabel}
                </button>
            )}

            {onOpenSettings === undefined || settingsLabel === undefined ? null : (
                <button className="trdr-inline-action" type="button" onClick={onOpenSettings}>
                    {settingsLabel}
                </button>
            )}
        </div>
    );
}
