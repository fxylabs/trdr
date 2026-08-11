import { useId } from "react";
import type { ReactNode } from "react";

export type ViewLeadProps =
{
    /** The `h2` under the view's `h1`. */
    headline: string;
    description?: string;
    /** A `<Tag/>`. What state the thing being led with is in. */
    status?: ReactNode;
    /** The lime-dotted pill. `Agent가 생성한 초안`. */
    pill?: string;
    /** Quiet text beside the pill. */
    eyebrow?: string;
};

/**
 * The one lead a view gets.
 *
 * A view has a single narrative opening — a headline, what it means, and the
 * state it is in — and the contract's rule is that it has exactly one. A second
 * `ViewLead` on a screen is the screen inventing a hierarchy the system already
 * has: everything below the lead is a `Panel` with a heading.
 */
export function ViewLead(props: ViewLeadProps)
{
    const { headline, description, status, pill, eyebrow } = props;
    const headlineId = useId();
    const hasRow = pill !== undefined || status !== undefined || eyebrow !== undefined;

    return (
        <section className="trdr-view-lead" data-state="ready" aria-labelledby={headlineId}>
            {!hasRow ? null : (
                <div className="trdr-view-lead-row">
                    {pill === undefined ? null : <span className="trdr-lead-pill">{pill}</span>}
                    {status}
                    {eyebrow === undefined ? null : <span className="trdr-muted">{eyebrow}</span>}
                </div>
            )}

            <h2 id={headlineId}>{headline}</h2>

            {description === undefined ? null : <p>{description}</p>}
        </section>
    );
}
