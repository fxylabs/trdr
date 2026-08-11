import { SHELL } from "../copy/ko";
import type { HostStatus } from "./useHostStatus";
import { useHostStatus } from "./useHostStatus";

/**
 * The one line that says which workspace this window is looking at.
 *
 * Minimal on purpose. Section 14's stop condition asks for evidence that the
 * WebView round-trips a typed command and gets real values back, and the
 * smallest honest evidence is the workspace id and the build version rendered
 * from the response rather than from a constant. Everything else a status bar
 * eventually carries — job state, collector health, the terminal's process —
 * arrives with the track that owns it.
 *
 * It sits at the foot of the sidebar rather than across the window, because the
 * visual contract's shell has three columns and no status row, and inventing a
 * fourth region would be a screen-local layout rule with a different name. What
 * it does need is to be smaller than body text, which is `<small>` doing what
 * `<small>` is for rather than a font size written down here.
 *
 * `role="status"` rather than a plain element: the content changes after the
 * first paint, and an assistive technology should hear that it did without the
 * change stealing focus. The label is what tells one live region on this screen
 * from another.
 */
export function StatusLine()
{
    const status = useHostStatus();

    return (
        <small className="trdr-muted" role="status" aria-label={SHELL.workspaceLabel} aria-live="polite">
            <Content status={status} />
        </small>
    );
}

function Content({ status }: { readonly status: HostStatus })
{
    if (status.kind === "connecting")
    {
        return <>{SHELL.connecting}</>;
    }

    if (status.kind === "unreachable")
    {
        return (
            <>
                {`${SHELL.unreachable} · `}
                <code className="trdr-num">{status.code}</code>
            </>
        );
    }

    return (
        <>
            {`${SHELL.workspaceLabel} `}
            <code className="trdr-num">{status.workspaceId}</code>
            {` · ${SHELL.versionLabel} `}
            <span className="trdr-num">{status.appVersion}</span>
        </>
    );
}
