import { useCallback, useEffect, useState } from "react";
import { useLocation } from "react-router";

import type { ErrorCode } from "../bindings";
import { navigation } from "../shell/routes";
import type { ProcessState } from "./processState";
import { INITIAL_PROCESS_STATE, liveProducer, nextFixture } from "./processState";
import type { TerminalOutcome, TerminalSession } from "./session";
import { TerminalStatusBar } from "./TerminalStatusBar";
import { TerminalTitleBar } from "./TerminalTitleBar";
import { XtermHost } from "./XtermHost";

/** What the title bar says before the host has answered with the real one. */
const UNKNOWN_AGENT = "agent";

/** What the rail is sized at before it has been measured. */
const UNMEASURED = { cols: 80, rows: 24 };

/**
 * The agent rail (the `TerminalShell` contract, and the `PersistentAgentRail`
 * recipe).
 *
 * Composes the three parts the contract names — the title bar, the xterm host,
 * and the status bar — and owns the one thing they share: the process state.
 *
 * Two rules from the contract hold here rather than being described:
 *
 * - The PTY and its scrollback survive centre-view navigation. Neither lives in
 *   this component: the session is a module-level value in `session.ts`, and the
 *   scrollback is a file the host owns. Navigating re-renders this and touches
 *   neither.
 * - Raw output is never turned into product components. Nothing in this file
 *   sees a byte. The bytes go from the session's channel into xterm, and the
 *   state comes from a separate channel that cannot carry one.
 *
 * # Where the styling comes from
 *
 * Every class here is `design/ui-kit/components.v2.css`'s, and not one of them
 * is defined in this app. The stylesheet is shipped as the `@trdr/ui` package,
 * and importing it belongs to whoever wires that package into this app's entry
 * point — it is one import, in one place, for every screen and not only for the
 * rail. Until it is there the markup is correct and unpainted, which is a state
 * a person can see and act on; a copy of the terminal's colours kept here so
 * that it looked right in the meantime would not be.
 */
export function TerminalShell({ session }: { session: TerminalSession })
{
    const [state, setState] = useState<ProcessState>(INITIAL_PROCESS_STATE);
    const [agent, setAgent] = useState(UNKNOWN_AGENT);
    const [size, setSize] = useState(UNMEASURED);
    const [failure, setFailure] = useState<ErrorCode | null>(null);
    const [focused, setFocused] = useState(false);
    const context = useCentreView();

    const measured = useCallback((cols: number, rows: number) =>
    {
        setSize({ cols, rows });
    }, []);

    const settle = useCallback((outcome: TerminalOutcome) =>
    {
        setFailure(outcome.kind === "error" ? outcome.code : null);
        setAgent((named) => nameFrom(outcome) ?? named);

        if (outcome.kind === "ok")
        {
            setSize({ cols: outcome.session.cols, rows: outcome.session.rows });
        }
    }, []);

    useEffect(
        () => liveProducer((handler) => session.onProcess(handler)).subscribe(setState),
        [session]
    );
    useEffect(() =>
    {
        let listening = true;

        void session.start().then((outcome) =>
        {
            if (listening)
            {
                settle(outcome);
            }
        });

        return () =>
        {
            listening = false;
        };
    }, [session, settle]);

    return (
        <div
            className="trdr-terminal-shell"
            data-process={state.process}
            data-focus={focused}
            onFocus={() => setFocused(true)}
            onBlur={() => setFocused(false)}
        >
            <TerminalTitleBar agent={agent} cols={size.cols} rows={size.rows} />
            <XtermHost session={session} onMeasured={measured} />
            <TerminalStatusBar
                state={state}
                context={context}
                failure={failure}
                onRestart={() => void session.restart().then(settle)}
                onStepFixture={() => setState((current) => nextFixture(current) ?? current)}
            />
        </div>
    );
}

/**
 * Which agent the host is talking about.
 *
 * A refusal names one too — `TERMINAL_EXECUTABLE_MISSING` carries the name it
 * could not find — so a rail whose agent is not installed still says which one
 * it was looking for, rather than going blank.
 */
function nameFrom(outcome: TerminalOutcome): string | null
{
    return outcome.kind === "ok" ? outcome.session.agent : outcome.agent;
}

/**
 * What the centre view is showing, for the status bar's context.
 *
 * Read from the route table rather than from the URL directly, so the words
 * here are the same words the navigation uses and there is one place to change
 * either.
 */
function useCentreView(): string
{
    const { pathname } = useLocation();

    return navigation.find((section) => section.path === pathname)?.label ?? pathname;
}
