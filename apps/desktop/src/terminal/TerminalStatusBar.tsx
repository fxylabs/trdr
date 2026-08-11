import type { ErrorCode } from "../bindings";
import type { ProcessState } from "./processState";
import { fixtureIsAvailable } from "./processState";

/** The shortcut the contract fixes for focusing the rail. */
const FOCUS_SHORTCUT = "⌘J";

interface Props
{
    /** The state, and where it came from. */
    readonly state: ProcessState;
    /** What the centre view is showing, as one short phrase. */
    readonly context: string;
    /** The code the last start or restart refused with, if it refused. */
    readonly failure: ErrorCode | null;
    /** Stops the agent and starts it again. */
    readonly onRestart: () => void;
    /** Moves the synthetic fixture to its next state. */
    readonly onStepFixture: () => void;
}

/**
 * The strip under the terminal (the `TerminalStatusBar` contract).
 *
 * # What is announced, and what is not
 *
 * The contract asks for `aria-live="polite"` on process-state changes and for
 * every terminal byte *not* to be announced. Both come from where the live
 * region is: it is the one element holding the state, and the terminal's own
 * subtree is nowhere inside it. A screen reader hears "running", then "exited",
 * and never hears the agent redraw a progress bar four times a second. Reading
 * the terminal itself is xterm's accessibility support, which is a different
 * surface with a different setting.
 *
 * # Why the state is the whole message
 *
 * Section 11 maps finite states and codes to words in the surface, and forbids
 * anything generated from writing a verdict. So this renders the state's own
 * name and, when a start was refused, the section 12 code — the same way the
 * shell's status line renders `APP_NOT_RUNNING`. There is no sentence here for
 * anything to leak into.
 *
 * # The synthetic control
 *
 * M2 runs the whole app on synthetic data and has to show all seven process
 * states. The stepper is how the two the host cannot observe are seen. It says
 * `synthetic` on it, the state it produces is marked `synthetic` beside it, and
 * it is disabled whenever a session is live — see `processState.ts` for why
 * that refusal is in the model rather than in this handler.
 */
export function TerminalStatusBar({
    state,
    context,
    failure,
    onRestart,
    onStepFixture
}: Props)
{
    return (
        <footer className="trdr-terminal-statusbar">
            <strong aria-live="polite">
                {state.process.toUpperCase()}
                {state.origin === "synthetic" ? " · SYNTHETIC" : ""}
            </strong>

            <span>context: {context}</span>

            {failure !== null && <span className="trdr-term-error"> · {failure}</span>}

            {state.process === "exited" && (
                <button type="button" onClick={onRestart}>
                    restart
                </button>
            )}

            <button
                type="button"
                onClick={onStepFixture}
                disabled={!fixtureIsAvailable(state)}
                title="Synthetic fixture: walks the seven process states while no agent is running"
            >
                synthetic: next state
            </button>

            <span>{FOCUS_SHORTCUT} focus</span>
        </footer>
    );
}
