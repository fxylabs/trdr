import type { ReactNode } from "react";

export type EmptyStateProps =
{
    title: string;
    /** Why it is empty. Not "no data" — what has not happened yet. */
    explanation: string;
    /** The single next step, as a `<Button/>`. */
    action?: ReactNode;
};

/**
 * A surface with nothing in it, explained.
 *
 * Two things, both required by the contract: the reason it is empty, and the one
 * step that would change that. An empty state offering three choices is a screen
 * that has not decided what the user came for.
 */
export function EmptyState(props: EmptyStateProps)
{
    const { title, explanation, action } = props;

    return (
        <div className="trdr-empty-state" data-state="empty">
            <div>
                <strong>{title}</strong>
                <p>{explanation}</p>
                {action}
            </div>
        </div>
    );
}
