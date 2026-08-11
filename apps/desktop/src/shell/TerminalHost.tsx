import { memo } from "react";

/** The id the host container carries, so a test and a later terminal can find it. */
export const TERMINAL_HOST_ID = "terminal-host";

/**
 * The container the agent terminal will attach to.
 *
 * Empty on purpose, and it stays empty until the track that owns the PTY brings
 * xterm with it. What this component contributes now is the one property that
 * cannot be added afterwards: a DOM node that outlives navigation.
 *
 * A terminal is not a view of state that can be thrown away and rebuilt. It is a
 * live process on the other end of a pipe, with scrollback, a cursor, and a
 * program that is mid-sentence. Unmounting the node it is attached to loses all
 * of that, and no amount of caching further up puts it back. So the node is
 * created once, by the layout route, and every screen change happens beside it.
 *
 * `React.memo` is not what keeps it mounted — reconciliation is, because
 * `AppShell` renders this in the same position on every render. The memo only
 * spares it the re-render, which matters once something expensive is attached.
 */
export const TerminalHost = memo(function TerminalHost()
{
    return (
        <section className="terminal-host" aria-label="Agent terminal">
            <div id={TERMINAL_HOST_ID} className="terminal-host__surface" />
        </section>
    );
});
