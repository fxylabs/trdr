/**
 * The one agent terminal, as the screen talks to it.
 *
 * # Two channels, and why they are channels
 *
 * `docs/FOUNDATION_DESIGN.md` section 9.1 keeps host-to-UI state events
 * separate from the terminal byte stream. Here that is two `Channel` objects,
 * created here and handed to `terminal.start`: `output` carries base64 chunks
 * and nothing else, `lifecycle` carries a process state and nothing else.
 *
 * They are channels rather than Tauri events for a reason the capability file
 * shows. An event listener needs `core:event:listen`, which is the general
 * ability to hear anything the host ever emits; a channel is created by this
 * side, passed to one command, and reaches nothing else. So
 * `capabilities/main.json` stays a list of the four terminal commands, and
 * `src-tauri/tests/capability.rs` keeps proving that `plugin:event|listen` is
 * out of reach.
 *
 * # Why the session is a module-level value
 *
 * A terminal is a live process with scrollback, a cursor, and a program that is
 * mid-sentence. It cannot belong to a component, because a component's lifetime
 * is a render tree's and this outlives every screen the person navigates
 * through. [`terminalSession`] is therefore made once, when the module is first
 * imported, and [`TerminalSession.start`] remembers its own promise — so React's
 * development double-invoke, a navigation, and a re-render all reach the same
 * session rather than starting a second agent.
 *
 * Nothing here reads a byte it carries. Section 11 forbids turning terminal
 * content into product state, and the only thing this file does with a chunk is
 * decode the base64 it travelled in and hand the bytes on.
 */

import { Channel } from "@tauri-apps/api/core";

import type {
    ErrorCode,
    ErrorParam,
    TerminalOutput,
    TerminalProcess,
    TerminalProcessEvent,
    TerminalSessionModel
} from "../bindings";
import { commands } from "../bindings";
import { newRequestId } from "../ipc/requestId";
import { decodeBase64, encodeBase64 } from "./bytes";

/** How a start or a restart went. */
export type TerminalOutcome =
    | { readonly kind: "ok"; readonly session: TerminalSessionModel }
    | { readonly kind: "error"; readonly code: ErrorCode; readonly agent: string | null };

/** Told about every chunk the agent printed. */
export type ByteListener = (bytes: Uint8Array) => void;

/** Told about every process-state change, and never about a byte. */
export type ProcessListener = (process: TerminalProcess) => void;

/** The agent terminal the host owns. */
export interface TerminalSession
{
    /**
     * Starts the agent, or answers with the start that already happened.
     *
     * Safe to call as often as anything likes: the promise is made once and
     * handed out again afterwards.
     */
    start(): Promise<TerminalOutcome>;

    /** Stops the agent and starts it again. */
    restart(): Promise<TerminalOutcome>;

    /** Sends bytes to the agent. */
    input(bytes: Uint8Array): void;

    /** Tells the host the terminal changed size. */
    resize(cols: number, rows: number): void;

    /** Listens to the byte stream. Returns the function that stops listening. */
    onOutput(listener: ByteListener): () => void;

    /** Listens to process states. Returns the function that stops listening. */
    onProcess(listener: ProcessListener): () => void;
}

/** A session with its own pair of channels, for a test that wants a fresh one. */
export function createTerminalSession(): TerminalSession
{
    const output = new Channel<TerminalOutput>();
    const lifecycle = new Channel<TerminalProcessEvent>();
    const byteListeners = new Set<ByteListener>();
    const processListeners = new Set<ProcessListener>();
    let started: Promise<TerminalOutcome> | null = null;

    output.onmessage = (message) =>
    {
        const chunk = decodeBase64(message.data_base64);

        for (const listener of byteListeners)
        {
            listener(chunk);
        }
    };

    lifecycle.onmessage = (message) =>
    {
        for (const listener of processListeners)
        {
            listener(message.process);
        }
    };

    const start = () =>
    {
        started ??= commands
            .terminalStart(newRequestId(), output, lifecycle)
            .then(outcomeOf)
            .catch(unreachableHost);

        return started;
    };

    return {
        start,

        // A restart goes through the start first, because the channels above
        // reach the host only once `terminal.start` has been given them. A
        // restart of a terminal that was never started would otherwise replace a
        // child nobody was listening to.
        restart: async () =>
        {
            await start();

            return commands
                .terminalRestart(newRequestId())
                .then(outcomeOf)
                .catch(unreachableHost);
        },

        input: (bytes) =>
        {
            void commands
                .terminalInput(newRequestId(), { data_base64: encodeBase64(bytes) })
                .catch(() => undefined);
        },

        resize: (cols, rows) =>
        {
            void commands.terminalResize(newRequestId(), { cols, rows }).catch(() => undefined);
        },

        onOutput: (listener) => subscribe(byteListeners, listener),
        onProcess: (listener) => subscribe(processListeners, listener)
    };
}

/**
 * The session the app uses.
 *
 * One per window, made when this module is first imported and never replaced.
 */
export const terminalSession = createTerminalSession();

/** Adds a listener and answers with the function that removes it. */
function subscribe<T>(listeners: Set<T>, listener: T): () => void
{
    listeners.add(listener);

    return () =>
    {
        listeners.delete(listener);
    };
}

/**
 * The envelope, read as an outcome.
 *
 * A command answers with an envelope and never rejects (see `commands.rs`), so
 * the failure arm here is a refusal the host wrote down rather than a broken
 * bridge — the agent is not installed, the spawn failed, the process is gone.
 */
function outcomeOf(
    answer: Awaited<ReturnType<typeof commands.terminalStart>>
): TerminalOutcome
{
    if (answer.outcome.status === "ok")
    {
        return { kind: "ok", session: answer.outcome.value };
    }

    return {
        kind: "error",
        code: answer.outcome.value.code,
        agent: agentNamedBy(answer.outcome.value.params)
    };
}

/**
 * Which agent an error was about, when it says.
 *
 * `TERMINAL_EXECUTABLE_MISSING` carries the configured name so the rail can say
 * which program it could not find. Section 12 allows only safe parameters here,
 * and the host sends the name from its own setting — never a resolved location
 * and never anything the agent printed.
 */
function agentNamedBy(params: { [key: string]: ErrorParam } | undefined): string | null
{
    const named = params?.["agent"];

    return named !== undefined && named.type === "text" ? named.value : null;
}

/** There is no host behind this WebView, which is the one code for that. */
function unreachableHost(): TerminalOutcome
{
    return { kind: "error", code: "APP_NOT_RUNNING", agent: null };
}
