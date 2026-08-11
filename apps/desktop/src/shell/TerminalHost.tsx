import { memo } from "react";

import { TerminalShell } from "../terminal/TerminalShell";
import { terminalSession } from "../terminal/session";

/** The id the host container carries, so a test and the terminal can find it. */
export const TERMINAL_HOST_ID = "terminal-host";

/**
 * The container the agent terminal is attached to.
 *
 * What this component contributes is the one property that cannot be added
 * afterwards: a DOM node that outlives navigation.
 *
 * A terminal is not a view of state that can be thrown away and rebuilt. It is a
 * live process on the other end of a pipe, with scrollback, a cursor, and a
 * program that is mid-sentence. Unmounting the node it is attached to loses all
 * of that, and no amount of caching further up puts it back. So the node is
 * created once, by the layout route, and every screen change happens beside it.
 *
 * `React.memo` is not what keeps it mounted — reconciliation is, because
 * `AppShell` renders this in the same position on every render. The memo spares
 * it the re-render, which now matters: a live terminal is attached below.
 *
 * # What is here and what is elsewhere
 *
 * The session is not this component's. `terminal/session.ts` holds it at module
 * level, because even this node is younger than the process it draws: the host
 * keeps the child and the scrollback, and a WebView that reloaded is replayed
 * what it missed. What is here is the placement, and `AppShell.test.tsx` and
 * `terminal/persistence.test.tsx` are what hold it.
 */
export const TerminalHost = memo(function TerminalHost()
{
    return (
        <section className="terminal-host" aria-label="Agent terminal">
            <div id={TERMINAL_HOST_ID} className="terminal-host__surface">
                <TerminalShell session={terminalSession()} />
            </div>
        </section>
    );
});
