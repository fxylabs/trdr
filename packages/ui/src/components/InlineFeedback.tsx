import type { ReactNode } from "react";

export type FeedbackTone = "neutral" | "success" | "warning" | "error";

export type InlineFeedbackProps =
{
    message: ReactNode;
    tone?: FeedbackTone;
    state?: "visible" | "dismissed";
    onRetry?: () => void;
    onDismiss?: () => void;
    retryLabel?: string;
    dismissLabel?: string;
};

/**
 * What went wrong here, and what to do about it here.
 *
 * An error tone announces itself — `role="alert"` — because a failure the user
 * did not see is a failure they will hit again. Everything else is a polite
 * status. What this never carries is the agent's own error output: that is raw
 * text in the terminal's scrollback and it stays legible there.
 */
export function InlineFeedback(props: InlineFeedbackProps)
{
    const { message, tone = "neutral", state = "visible" } = props;
    const { onRetry, onDismiss, retryLabel, dismissLabel } = props;

    if (state === "dismissed")
    {
        return null;
    }

    return (
        <div
            className="trdr-inline-feedback"
            data-state={state}
            data-tone={tone}
            role={tone === "error" ? "alert" : "status"}
        >
            {message}

            {onRetry === undefined || retryLabel === undefined ? null : (
                <button className="trdr-inline-action" type="button" onClick={onRetry}>
                    {retryLabel}
                </button>
            )}

            {onDismiss === undefined || dismissLabel === undefined ? null : (
                <button className="trdr-inline-action" type="button" onClick={onDismiss}>
                    {dismissLabel}
                </button>
            )}
        </div>
    );
}
