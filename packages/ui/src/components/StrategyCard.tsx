import type { MouseEvent, ReactNode } from "react";

export type StrategyCardState = "ready" | "running" | "completed" | "discarded";

export type StrategyStat =
{
    label: string;
    /** Formatted. `8 / 20일`, `이탈 1`, `+8.4%`. */
    value: string;
    note?: string;
};

export type StrategySummary =
{
    id: string;
    name: string;
    description?: string;
    /** A `<Tag/>`. Where the validation is: `모의검증 중 · D−12`. */
    status?: ReactNode;
    /** How far the observation has got. The first statistic, always. */
    observation: StrategyStat;
    /** The paper return. Same weight as its neighbours, and never coloured. */
    paperReturn: StrategyStat;
    /** Rule deviations, breaches, whatever the strategy is doing now. */
    currentState: StrategyStat;
};

export type StrategyCardProps =
{
    strategy: StrategySummary;
    state?: StrategyCardState;
    /** Given, the card is a link; omitted, a button. Either way it is one control. */
    href?: string;
    onOpen?: (strategyId: string) => void;
};

/**
 * One strategy in the list, as a single link.
 *
 * The layout is the contract's rule, and it is a claim about what matters. The
 * dominant column is the strategy's name and what it is; the three statistics
 * beside it are the same size as each other, and the first of them is how much
 * of the observation window has actually elapsed. The return is in the middle,
 * uncoloured — a paper return over eight of twenty days is not yet evidence, and
 * a card that shows it in green and large says otherwise before the user has
 * read a word.
 */
export function StrategyCard(props: StrategyCardProps)
{
    const { strategy, state = "ready", href, onOpen } = props;

    const body = (
        <>
            <div className="trdr-strategy-name">
                {strategy.status}
                <h3>{strategy.name}</h3>
                {strategy.description === undefined ? null : <p>{strategy.description}</p>}
            </div>

            <Stat stat={strategy.observation} />
            <Stat stat={strategy.paperReturn} />
            <Stat stat={strategy.currentState} />

            <b aria-hidden="true">›</b>
        </>
    );

    if (href !== undefined)
    {
        return (
            <a
                className="trdr-strategy-card"
                data-state={state}
                href={href}
                onClick={(event: MouseEvent<HTMLAnchorElement>) =>
                {
                    if (onOpen !== undefined)
                    {
                        event.preventDefault();
                        onOpen(strategy.id);
                    }
                }}
            >
                {body}
            </a>
        );
    }

    return (
        <button
            className="trdr-strategy-card"
            data-state={state}
            type="button"
            onClick={onOpen === undefined ? undefined : () => onOpen(strategy.id)}
        >
            {body}
        </button>
    );
}

function Stat(props: { stat: StrategyStat })
{
    const { label, value, note } = props.stat;

    return (
        <div className="trdr-strategy-stat">
            <label>{label}</label>
            <strong className="trdr-num">{value}</strong>
            {note === undefined ? null : <small>{note}</small>}
        </div>
    );
}
