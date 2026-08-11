import { useId } from "react";
import type { ReactNode } from "react";

/** `rest` is a panel with something in it; the rest are what it is waiting on. */
export type PanelState = "rest" | "loading" | "empty" | "error";

export type PanelProps =
{
    title?: string;
    /** The quiet note beside the title. `strategy.yaml`, `52px data rows`. */
    caption?: string;
    /** Pushed to the right of the header — a `<Tag/>`, a `<Button/>`. */
    end?: ReactNode;
    state?: PanelState;
    children?: ReactNode;
};

/**
 * A grouping surface.
 *
 * Not a wrapper. A `Panel` says "these things belong together and this is what
 * they are called", and a screen that puts one around every element has said
 * nothing while spending a border on it. Nothing in this package wraps itself in
 * a `Panel` either — `Metric` and `DataTable` render their own role and let the
 * screen decide whether the group around them is a named one.
 */
export function Panel(props: PanelProps)
{
    const { title, caption, end, state = "rest", children } = props;
    const titleId = useId();

    const header =
        title === undefined ? null : (
            <header className="trdr-panel-header">
                <h3 id={titleId}>{title}</h3>
                {caption === undefined ? null : <span>{caption}</span>}
                {end === undefined ? null : <span className="trdr-panel-header-end">{end}</span>}
            </header>
        );

    if (title === undefined)
    {
        return (
            <div className="trdr-panel" data-state={state}>
                {children}
            </div>
        );
    }

    return (
        <section className="trdr-panel" data-state={state} aria-labelledby={titleId}>
            {header}
            {children}
        </section>
    );
}
