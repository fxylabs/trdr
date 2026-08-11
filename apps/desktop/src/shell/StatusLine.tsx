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
 * `role="status"` rather than a plain element: the content changes after the
 * first paint, and an assistive technology should hear that it did without the
 * change stealing focus.
 */
export function StatusLine()
{
    const status = useHostStatus();

    return (
        <footer className="shell__status" role="status" aria-live="polite">
            <Content status={status} />
        </footer>
    );
}

function Content({ status }: { readonly status: HostStatus })
{
    if (status.kind === "connecting")
    {
        return <span className="shell__status-note">asking the host…</span>;
    }

    if (status.kind === "unreachable")
    {
        return (
            <span className="shell__status-note">
                the host did not answer <code>{status.code}</code>
            </span>
        );
    }

    return (
        <>
            <span className="shell__status-label">workspace</span>
            <code className="shell__status-value">{status.workspaceId}</code>
            <span className="shell__status-label">trdr</span>
            <span className="shell__status-value">{status.appVersion}</span>
        </>
    );
}
