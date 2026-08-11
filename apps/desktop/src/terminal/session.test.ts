import { beforeEach, expect, test, vi } from "vitest";

import type { TerminalOutput, TerminalProcessEvent } from "../bindings";
import { commands } from "../bindings";
import { createTerminalSession } from "./session";
import { decodeBase64, encodeBase64 } from "./bytes";

/**
 * A channel with no WebView under it.
 *
 * The real one asks Tauri's internals for a callback id the moment it is
 * constructed, which there is nothing here to answer. What is being tested is
 * what the session does with a message, so the message is delivered directly.
 */
vi.mock("@tauri-apps/api/core", () => ({
    Channel: class
    {
        public onmessage: ((message: unknown) => void) | null = null;
    }
}));

vi.mock("../bindings", () => ({
    commands: {
        terminalStart: vi.fn(),
        terminalInput: vi.fn(),
        terminalResize: vi.fn(),
        terminalRestart: vi.fn()
    }
}));

const REQUEST = "01KZNNR5X818P3J6ENYKSADP8W";

function started(agent = "claude")
{
    return {
        v: 1,
        id: REQUEST,
        outcome: {
            status: "ok" as const,
            value: { agent, pid: 4242, process: "ready" as const, cols: 112, rows: 42 }
        }
    };
}

/** The channels the session handed to `terminal.start`, as the host holds them. */
function channels()
{
    const call = vi.mocked(commands.terminalStart).mock.calls[0];

    if (call === undefined)
    {
        throw new Error("terminal.start was never called");
    }

    return {
        output: call[1] as unknown as { onmessage: ((m: TerminalOutput) => void) | null },
        lifecycle: call[2] as unknown as { onmessage: ((m: TerminalProcessEvent) => void) | null }
    };
}

beforeEach(() =>
{
    vi.resetAllMocks();
    vi.mocked(commands.terminalStart).mockResolvedValue(started());
    vi.mocked(commands.terminalRestart).mockResolvedValue(started());
    vi.mocked(commands.terminalInput).mockResolvedValue({
        v: 1,
        id: REQUEST,
        outcome: { status: "ok", value: { process: "running" } }
    });
    vi.mocked(commands.terminalResize).mockResolvedValue({
        v: 1,
        id: REQUEST,
        outcome: { status: "ok", value: { process: "running" } }
    });
});

/**
 * The property that makes navigation harmless. React's development strict mode
 * runs an effect twice, a re-render runs it again, and none of that may start a
 * second agent.
 */
test("starting many times starts one agent", async () =>
{
    const session = createTerminalSession();

    const [first, second, third] = await Promise.all([
        session.start(),
        session.start(),
        session.start()
    ]);

    expect(commands.terminalStart).toHaveBeenCalledTimes(1);
    expect(first).toEqual(second);
    expect(second).toEqual(third);
});

/** Bytes arrive as bytes, escapes and invalid UTF-8 included. */
test("a chunk reaches the listener as the bytes the agent printed", async () =>
{
    const session = createTerminalSession();
    const seen: Uint8Array[] = [];
    session.onOutput((bytes) => seen.push(bytes));
    await session.start();

    const printed = new Uint8Array([0x1b, 0x5b, 0x33, 0x31, 0x6d, 0xff, 0xfe]);
    channels().output.onmessage?.({ data_base64: encodeBase64(printed) });

    expect(seen).toEqual([printed]);
});

/**
 * The two channels are two channels. A state change carries a state and reaches
 * the process listeners; it never reaches the byte listeners, and there is no
 * field on it for a byte to travel in.
 */
test("a process state reaches the state listener and never the byte listener", async () =>
{
    const session = createTerminalSession();
    const bytes: Uint8Array[] = [];
    const states: string[] = [];
    session.onOutput((chunk) => bytes.push(chunk));
    session.onProcess((process) => states.push(process));
    await session.start();

    channels().lifecycle.onmessage?.({ process: "running" });

    expect(states).toEqual(["running"]);
    expect(bytes).toEqual([]);
});

test("what is typed crosses as base64 of the bytes, not as text", () =>
{
    const session = createTerminalSession();

    session.input(new Uint8Array([0x1b, 0xff, 0x0d]));

    const call = vi.mocked(commands.terminalInput).mock.calls[0];
    expect(decodeBase64(call?.[1].data_base64 ?? "")).toEqual(
        new Uint8Array([0x1b, 0xff, 0x0d])
    );
});

test("a resize is sent as the size it measured", () =>
{
    createTerminalSession().resize(112, 42);

    expect(commands.terminalResize).toHaveBeenCalledWith(expect.any(String), {
        cols: 112,
        rows: 42
    });
});

/**
 * The agent is not installed. The code and the name it could not find both come
 * out of the envelope, so the rail can say which one rather than going blank.
 */
test("a refused start is read out of the envelope it came in", async () =>
{
    vi.mocked(commands.terminalStart).mockResolvedValue({
        v: 1,
        id: REQUEST,
        outcome: {
            status: "error",
            value: {
                v: 1,
                code: "TERMINAL_EXECUTABLE_MISSING",
                params: { agent: { type: "text", value: "codex" } },
                retryability: { kind: "immediate" }
            }
        }
    });

    expect(await createTerminalSession().start()).toEqual({
        kind: "error",
        code: "TERMINAL_EXECUTABLE_MISSING",
        agent: "codex"
    });
});

/** No host behind the WebView at all, which is a different failure. */
test("a bridge that is not there is named rather than left pending", async () =>
{
    vi.mocked(commands.terminalStart).mockRejectedValue(new Error("no host here"));

    expect(await createTerminalSession().start()).toEqual({
        kind: "error",
        code: "APP_NOT_RUNNING",
        agent: null
    });
});

/**
 * A restart goes through the start first. The channels reach the host only once
 * `terminal.start` has been given them, so restarting a terminal that was never
 * started would otherwise replace a child nobody was listening to.
 */
test("a restart attaches the channels before it replaces the child", async () =>
{
    await createTerminalSession().restart();

    expect(commands.terminalStart).toHaveBeenCalledTimes(1);
    expect(commands.terminalRestart).toHaveBeenCalledTimes(1);
});
