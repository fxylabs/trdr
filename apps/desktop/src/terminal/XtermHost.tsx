import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import { useEffect, useRef } from "react";

// xterm's own stylesheet, which positions the rows, the cursor, and the
// selection. It carries no colours — those come from the theme below, out of
// the token block — so importing it does not put a second palette in the app.
// It is imported here rather than from the app's entry point so that the whole
// of the terminal, its markup and the stylesheet it needs, arrives together.
import "@xterm/xterm/css/xterm.css";

import type { TerminalSession } from "./session";
import { binaryBytes, typedBytes } from "./bytes";
import { terminalAppearance, tokensOf } from "./theme";

/** How many lines xterm keeps. The other end of the host's scrollback bound. */
const SCROLLBACK_LINES = 10_000;

/**
 * The node xterm draws into (the `XtermHost` contract).
 *
 * # Why the terminal is built once and never disposed here
 *
 * The obvious effect — build on mount, dispose on cleanup — is wrong for this
 * component, and wrong in a way that only shows up in development. React's
 * strict mode runs every effect twice: mount, clean up, mount again. A terminal
 * that disposed itself in cleanup would throw away its buffer, its cursor, and
 * everything the agent has drawn, and the second mount would come up blank
 * beside a process that is mid-sentence.
 *
 * So the two lifetimes are separated. The terminal is built on the first run
 * and kept on a ref; the listeners are added on every run and removed by that
 * run's own cleanup. Nothing accumulates, and nothing is thrown away.
 *
 * This component is mounted once for the life of the window — `AppShell` is a
 * layout route and renders it in the same position on every render — so there
 * is no real unmount for a disposal to belong to. `shell/TerminalHost.tsx` is
 * where that placement is explained, and `AppShell.test.tsx` is what holds it.
 *
 * # The bytes are bytes
 *
 * `terminal.write` is given a `Uint8Array`, not a string. xterm decodes UTF-8
 * incrementally when it is handed bytes, so a character split across two chunks
 * is put back together by the terminal that is going to draw it. Nothing on
 * this path looks at what the bytes say.
 */
interface Props
{
    /** The terminal this draws. */
    readonly session: TerminalSession;
    /** Told the grid's size whenever it changes, for the title bar to show. */
    readonly onMeasured: (cols: number, rows: number) => void;
}

export function XtermHost({ session, onMeasured }: Props)
{
    const node = useRef<HTMLDivElement | null>(null);
    const terminal = useRef<Terminal | null>(null);
    const fit = useRef<FitAddon | null>(null);

    useEffect(() =>
    {
        const host = node.current;

        if (host === null)
        {
            return undefined;
        }

        if (terminal.current === null)
        {
            const built = build(host);
            terminal.current = built.terminal;
            fit.current = built.fit;
        }

        return attach(terminal.current, fit.current, session, onMeasured);
    }, [session, onMeasured]);

    return <div className="trdr-xterm-host" ref={node} data-testid="xterm-host" />;
}

/** A terminal on this node, themed from the token block. */
function build(host: HTMLDivElement)
{
    const { theme, fontFamily, lineHeight } = terminalAppearance(tokensOf(host));
    const terminal = new Terminal({
        scrollback: SCROLLBACK_LINES,
        theme,
        ...(fontFamily === undefined ? {} : { fontFamily }),
        ...(lineHeight === undefined ? {} : { lineHeight })
    });
    const fit = new FitAddon();

    terminal.loadAddon(fit);
    terminal.open(host);

    return { terminal, fit };
}

/**
 * Wires one run of the effect to the session, and answers with its own undo.
 *
 * Every listener added here is removed by the returned function, and the
 * terminal itself outlives both.
 */
function attach(
    terminal: Terminal,
    fit: FitAddon | null,
    session: TerminalSession,
    onMeasured: (cols: number, rows: number) => void
): () => void
{
    const last = { cols: 0, rows: 0 };
    const measure = () => report(terminal, fit, session, onMeasured, last);
    const stopListening = session.onOutput((bytes) => terminal.write(bytes));
    const typed = terminal.onData((data) => session.input(typedBytes(data)));
    const binary = terminal.onBinary((data) => session.input(binaryBytes(data)));
    const observer = watch(terminal, measure);

    // Measured twice on purpose. The first runs while the grid is still
    // settling and its row count is one too many often enough to clip the
    // bottom line; the second runs after the browser has laid the frame out,
    // which is the first moment the rail's height is the height it will keep.
    measure();
    const settled = requestAnimationFrame(measure);

    return () =>
    {
        cancelAnimationFrame(settled);
        stopListening();
        typed.dispose();
        binary.dispose();
        observer?.disconnect();
    };
}

/** Watches the rail for the layout changes that resize the terminal. */
function watch(terminal: Terminal, measure: () => void): ResizeObserver | null
{
    const host = terminal.element?.parentElement;

    if (host === null || host === undefined || typeof ResizeObserver === "undefined")
    {
        return null;
    }

    const observer = new ResizeObserver(measure);
    observer.observe(host);

    return observer;
}

/**
 * Fits the grid to the rail and tells the host what size it came out at.
 *
 * The size goes to the host whether or not a child is running. A terminal is
 * measured while the shell is still laying itself out, which is before
 * `terminal.start` has returned, and again after the agent has exited — both
 * are the right size and neither has anywhere to be delivered yet. The host
 * keeps the last one and spawns the next child at it, so the agent's first
 * frame is already the right shape instead of being corrected one frame later.
 *
 * Only a size that changed is sent. A resize observer fires for every layout
 * pass the rail is caught up in, and most of them leave the grid at the same
 * number of cells; a command per pass would be a stream of `SIGWINCH` at a
 * child that has nothing to redraw.
 */
function report(
    terminal: Terminal,
    fit: FitAddon | null,
    session: TerminalSession,
    onMeasured: (cols: number, rows: number) => void,
    last: { cols: number; rows: number }
)
{
    try
    {
        fit?.fit();
    }
    catch
    {
        // `fit` measures a character cell, which needs a laid-out element. In a
        // detached or zero-sized rail there is nothing to measure, and the size
        // below is the last one that worked — which is the right answer anyway.
    }

    if (terminal.cols === last.cols && terminal.rows === last.rows)
    {
        return;
    }

    last.cols = terminal.cols;
    last.rows = terminal.rows;
    session.resize(terminal.cols, terminal.rows);
    onMeasured(terminal.cols, terminal.rows);
}
