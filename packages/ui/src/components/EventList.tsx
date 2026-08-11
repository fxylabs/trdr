import type { ReactNode } from "react";

export type EventListState = "loading" | "ready" | "empty" | "error";

export type EventContent =
{
    /** The event's type, in characters. `공시`, `전략`, `D−12`. Read, not hidden. */
    icon: ReactNode;
    title: ReactNode;
    summary: ReactNode;
};

export type EventListProps<Item> =
{
    /** The list's accessible name. */
    label: string;
    items: readonly Item[];
    itemId: (item: Item) => string;
    /** One item to the three parts the contract draws. */
    event: (item: Item) => EventContent;
    state?: EventListState;
    /** An `<EmptyState/>`. */
    empty?: ReactNode;
    onActivateEvent?: (id: string) => void;
};

/**
 * What happened, in the order it happened.
 *
 * Generic over the item for the same reason `DataTable` is: a disclosure, a
 * strategy signal and a validation milestone are three different records that
 * read as one line each, and the mapper is where a screen says which is which.
 *
 * The icon tile is text and stays in the reading order — the contract wants the
 * event's type available to someone who is not looking at the colour of a
 * square. Today's list carries only what the account is in: this component
 * cannot enforce that, and the screen filling it has to.
 */
export function EventList<Item>(props: EventListProps<Item>)
{
    const { label, items, itemId, event, state = "ready", empty, onActivateEvent } = props;

    return (
        <div
            className="trdr-event-list"
            data-state={state}
            role={state === "empty" ? undefined : "list"}
            aria-label={label}
        >
            {state === "loading"
                ? [0, 1, 2].map((row) => (
                      <div key={row} className="trdr-event" role="listitem">
                          <span className="trdr-event-icon" />
                          <i className="trdr-skeleton trdr-skeleton-line" />
                      </div>
                  ))
                : null}

            {state === "empty" ? empty : null}

            {state === "loading" || state === "empty"
                ? null
                : items.map((item) => (
                      <Event
                          key={itemId(item)}
                          content={event(item)}
                          {...(onActivateEvent === undefined
                              ? {}
                              : { onActivate: () => onActivateEvent(itemId(item)) })}
                      />
                  ))}
        </div>
    );
}

function Event(props: { content: EventContent; onActivate?: () => void })
{
    const { content, onActivate } = props;

    return (
        <div className="trdr-event" role="listitem">
            <span className="trdr-event-icon">{content.icon}</span>

            <div>
                <b>
                    {onActivate === undefined ? (
                        content.title
                    ) : (
                        <button className="trdr-inline-action" type="button" onClick={onActivate}>
                            {content.title}
                        </button>
                    )}
                </b>
                <p>{content.summary}</p>
            </div>
        </div>
    );
}
