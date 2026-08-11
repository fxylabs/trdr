/** The three window dots, in the order the gallery draws them. */
const DOTS = ["red", "amber", "green"] as const;

interface Props
{
    /** The agent as the host's setting names it. */
    readonly agent: string;
    /** Columns the child was last told about. */
    readonly cols: number;
    /** Rows the child was last told about. */
    readonly rows: number;
}

/**
 * The chrome above the terminal (the `TerminalTitleBar` contract).
 *
 * The rule this component exists to hold: the title chrome is trdr's and the
 * terminal content is the agent CLI's. Everything here is written by the app —
 * the dots, the agent's name, the transport, the size — and not one character
 * of it comes from the stream below.
 *
 * The dots are decorative and marked so; there is nothing behind them to
 * activate, and a screen reader announcing "red amber green" before every
 * reading of the terminal would be noise.
 */
export function TerminalTitleBar({ agent, cols, rows }: Props)
{
    return (
        <header className="trdr-terminal-titlebar">
            {DOTS.map((colour) => (
                <span key={colour} className="trdr-window-dot" data-color={colour} aria-hidden="true" />
            ))}
            <strong>trdr — {agent}</strong>
            <span>
                PTY · raw · {cols}×{rows}
            </span>
        </header>
    );
}
