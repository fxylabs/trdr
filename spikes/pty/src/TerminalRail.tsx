import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import { useEffect, useRef } from "react";

import * as pty from "./pty";

interface Props
{
    hidden: boolean;
    onSize: (size: pty.Size) => void;
}

// Mounted once, by App, outside the route switch. That placement is the whole
// experiment: if this component ever unmounts on a route change, the xterm
// instance and its scrollback go with it, and no amount of host-side buffering
// hides the flicker. Routes may hide the rail, never unmount it.
//
// The effect still tears down completely, because it has to survive React's
// development double-invoke. Guarding on "already built" instead would leave the
// first run's cleanup to strip the listeners off a terminal the guard then
// refuses to rebuild, and the rail would come up looking alive and be deaf.
export function TerminalRail({ hidden, onSize }: Props)
{
    const host = useRef<HTMLDivElement | null>(null);
    const term = useRef<Terminal | null>(null);
    const fit = useRef<FitAddon | null>(null);
    const report = useRef(onSize);
    report.current = onSize;

    useEffect(() =>
    {
        if (host.current === null)
        {
            return;
        }
        const built = build(host.current);
        term.current = built.terminal;
        fit.current = built.fit;
        const stop = attach(built.terminal, built.fit, report);
        return () =>
        {
            void stop();
            built.terminal.dispose();
            term.current = null;
            fit.current = null;
        };
    }, []);

    useEffect(() =>
    {
        if (!hidden && fit.current !== null && term.current !== null)
        {
            refit(fit.current, term.current, report);
        }
    }, [hidden]);

    return <div className="rail" style={{ display: hidden ? "none" : "flex" }}>
        <div className="rail-label">agent — raw pty</div>
        <div className="rail-host" ref={host} />
    </div>;
}

type Reporter = { current: (size: pty.Size) => void };

function build(host: HTMLDivElement)
{
    const terminal = new Terminal({
        fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
        fontSize: 12,
        scrollback: 10000,
        theme: { background: "#0b0d10", foreground: "#d6dde5" }
    });
    const fit = new FitAddon();
    terminal.loadAddon(fit);
    terminal.open(host);
    return { terminal, fit };
}

// Scrollback is replayed from the host before the live stream is attached, so a
// restart shows what the previous run printed. It is written as bytes, exactly
// as it was stored.
function attach(terminal: Terminal, fit: FitAddon, report: Reporter)
{
    let live = true;
    const show = (bytes: Uint8Array) => { if (live) { terminal.write(bytes); } };
    const listeners = [
        pty.onOutput(show),
        pty.onClosed(() => { if (live) { terminal.write("\r\n[process exited]\r\n"); } })
    ];
    void pty.loadScrollback().then((saved) => { if (saved.length > 0) { show(pty.decode(saved)); } });
    const typed = terminal.onData((data) => { void pty.write(data).catch(() => undefined); });
    const observer = new ResizeObserver(() => { if (live) { refit(fit, terminal, report); } });
    observer.observe(host(terminal));
    // Measured twice on purpose. The first fit runs while the grid is still
    // settling, and its row count is one too many often enough that the bottom
    // line renders half-clipped against `overflow: hidden`. The second runs
    // after the browser has laid the frame out, which is the first moment the
    // rail's height is the height it will keep.
    refit(fit, terminal, report);
    const settled = requestAnimationFrame(() => { if (live) { refit(fit, terminal, report); } });
    return async () =>
    {
        live = false;
        cancelAnimationFrame(settled);
        typed.dispose();
        observer.disconnect();
        for (const stop of await Promise.all(listeners))
        {
            stop();
        }
    };
}

function host(terminal: Terminal): Element
{
    return terminal.element?.parentElement ?? document.body;
}

// A child sizes its own drawing from the tty it was handed, so a spawn at a
// guessed size is not the same as a spawn at the right size followed by a
// correction: the first frame is already wrong, and a TUI that has not yet
// installed its resize handler never redraws it. The measured size is reported
// upward so the spawn can use it.
function refit(fit: FitAddon, terminal: Terminal, report: Reporter)
{
    fit.fit();
    report.current({ cols: terminal.cols, rows: terminal.rows });
    void pty.resize(terminal.cols, terminal.rows).catch(() => undefined);
}
