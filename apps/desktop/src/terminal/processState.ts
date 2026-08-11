/**
 * One process state, and the two things that can produce it.
 *
 * The visual contract has seven terminal process states. The host can observe
 * five of them, and the other two cannot be produced honestly in M2:
 *
 * - `needs-input` would have to be read out of the agent's own output, and
 *   section 11 forbids turning terminal content into product state. There is no
 *   prompt detector in trdr and there is not going to be one.
 * - `approval-pending` belongs to the approval round-trip in section 9.3, which
 *   is milestone M6. It will be produced by the approval broker, which knows
 *   about pending requests, and not by the terminal, which does not.
 *
 * M2 still has to show all seven, because the whole app in M2 runs on synthetic
 * data (`docs/IMPLEMENTATION_PLAN.md`, M2). So the state is one value with a
 * declared producer, and there are two producers:
 *
 * | producer | where its states come from | when it can produce one |
 * |---|---|---|
 * | [`liveProducer`] | the host's `lifecycle` channel | always |
 * | [`nextFixture`] | [`FIXTURE_STATES`], written down here | only while nothing is running |
 *
 * # What stops a synthetic state from being mistaken for a real one
 *
 * Two things, and neither is a convention:
 *
 * - The origin travels with the value. A [`ProcessState`] is a state *and*
 *   where it came from, so a surface cannot render one without being able to
 *   say which it is — and [`TerminalStatusBar`](./TerminalStatusBar) marks
 *   every synthetic one.
 * - The fixture producer is refused while a session is live. [`nextFixture`]
 *   answers with nothing unless the last live state was `exited`, so there is no
 *   switch a running session could be pushed through into a state nobody
 *   observed. The live producer's states always take effect.
 */

import type { TerminalProcess } from "../bindings";

/** Where a process state came from. */
export type ProcessOrigin = "live" | "synthetic";

/** A process state, and where it came from. */
export interface ProcessState
{
    /** The state itself. */
    readonly process: TerminalProcess;
    /** Whether the host observed it, or the fixture made it up. */
    readonly origin: ProcessOrigin;
}

/**
 * Something that produces process states.
 *
 * `subscribe` starts it and answers with the function that stops it, which is
 * the shape a React effect returns directly.
 */
export interface ProcessStateProducer
{
    /** Starts producing. */
    subscribe(publish: (state: ProcessState) => void): () => void;
}

/** The seven states, in the order the fixture walks them. */
export const FIXTURE_STATES: readonly TerminalProcess[] = [
    "starting",
    "ready",
    "running",
    "needs-input",
    "approval-pending",
    "exited",
    "reconnecting"
];

/** What the rail shows before anything has been observed or stepped through. */
export const INITIAL_PROCESS_STATE: ProcessState = {
    process: "exited",
    origin: "live"
};

/**
 * The live producer: whatever the host says about its own child.
 *
 * Takes the session's `onProcess` rather than the session, because that is the
 * whole of what it needs and a producer that could also write to the terminal
 * would be a producer that could do more than produce.
 */
export function liveProducer(
    listen: (handler: (process: TerminalProcess) => void) => () => void
): ProcessStateProducer
{
    return {
        subscribe: (publish) => listen((process) => publish({ process, origin: "live" }))
    };
}

/**
 * The next state the fixture would show, or nothing because it may not.
 *
 * The refusal is the point. A synthetic state can only follow a state the host
 * reported as `exited`, so a session with a child in it cannot be stepped into
 * a state nobody observed — however the control that calls this is wired, and
 * whatever a later screen does with it.
 */
export function nextFixture(current: ProcessState): ProcessState | null
{
    if (current.origin === "live" && current.process !== "exited")
    {
        return null;
    }

    const at = FIXTURE_STATES.indexOf(current.process);

    return {
        process: FIXTURE_STATES[(at + 1) % FIXTURE_STATES.length] ?? "starting",
        origin: "synthetic"
    };
}

/**
 * Whether the fixture may produce anything at all right now.
 *
 * What a control renders as `disabled`, so that the reason it cannot be used is
 * on the screen rather than only in the handler.
 */
export function fixtureIsAvailable(current: ProcessState): boolean
{
    return nextFixture(current) !== null;
}
