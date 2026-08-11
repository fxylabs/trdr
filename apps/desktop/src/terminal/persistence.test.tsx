import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { RouterProvider, createMemoryRouter } from "react-router";
import { afterEach, beforeEach, expect, test, vi } from "vitest";

import { commands } from "../bindings";
import { paths, routes } from "../shell/routes";
import { TERMINAL_HOST_ID } from "../shell/TerminalHost";
import { encodeBase64 } from "./bytes";

/**
 * What node identity is not enough to prove.
 *
 * `AppShell.test.tsx` holds that React keeps the terminal host's DOM node
 * across a navigation, which is necessary and not sufficient: xterm can be
 * disposed and rebuilt on the same node, and the rail would look identical
 * while having lost the buffer, the cursor, and — if the session were also
 * rebuilt — the agent process itself.
 *
 * This file is the rest of the proof. It navigates the same route table the app
 * uses and asserts three things afterwards:
 *
 * - the host was asked to start the agent exactly once, so the process on the
 *   other end is the same process;
 * - the terminal is the same instance, so it was never disposed;
 * - everything the agent printed is still in it, so the scrollback survived.
 */

/** As much of a terminal as the assertions below need. */
interface TerminalDouble
{
    disposed: boolean;
    readonly written: Uint8Array[];
}

/**
 * Where the terminals built during one test are recorded.
 *
 * `vi.hoisted` because `vi.mock` is lifted above every import, so the factory
 * below runs before anything an ordinary `const` declared would exist.
 */
const registry = vi.hoisted(() => ({ built: [] as TerminalDouble[] }));

vi.mock("@xterm/xterm", () => ({
    Terminal: class
    {
        public cols = 112;

        public rows = 42;

        public element: HTMLElement | null = null;

        public disposed = false;

        public readonly written: Uint8Array[] = [];

        public constructor()
        {
            registry.built.push(this);
        }

        public loadAddon(): void {}

        public open(host: HTMLElement): void
        {
            this.element = document.createElement("div");
            host.append(this.element);
        }

        public write(bytes: Uint8Array): void
        {
            this.written.push(bytes);
        }

        public onData(): { dispose: () => void }
        {
            return { dispose: () => undefined };
        }

        public onBinary(): { dispose: () => void }
        {
            return { dispose: () => undefined };
        }

        public dispose(): void
        {
            this.disposed = true;
        }
    }
}));

vi.mock("@xterm/addon-fit", () => ({
    FitAddon: class
    {
        public fit(): void {}
    }
}));

vi.mock("@xterm/xterm/css/xterm.css", () => ({}));

vi.mock("@tauri-apps/api/core", () => ({
    Channel: class
    {
        public onmessage: ((message: unknown) => void) | null = null;
    }
}));

vi.mock("../bindings", () => ({
    commands: {
        ping: vi.fn(),
        bootstrapGet: vi.fn(),
        terminalStart: vi.fn(),
        terminalInput: vi.fn(),
        terminalResize: vi.fn(),
        terminalRestart: vi.fn()
    }
}));

const REQUEST = "01KZNNR5X818P3J6ENYKSADP8W";
const WORKSPACE = "01KZNP0GQ3X8ARK9DQ489Z7WJ8";

function acknowledged()
{
    return {
        v: 1,
        id: REQUEST,
        outcome: { status: "ok" as const, value: { process: "running" as const } }
    };
}

beforeEach(() =>
{
    registry.built.length = 0;
    vi.mocked(commands.ping).mockResolvedValue({
        v: 1,
        id: REQUEST,
        outcome: { status: "ok", value: { protocol_version: 1 } }
    });
    vi.mocked(commands.bootstrapGet).mockResolvedValue({
        v: 1,
        id: REQUEST,
        outcome: {
            status: "ok",
            value: {
                protocol_version: 1,
                app_version: "9.9.9",
                workspace_id: WORKSPACE,
                workspace_path: "/private/tmp/trdr-t/x/workspaces/default",
                schema_version: 1
            }
        }
    });
    vi.mocked(commands.terminalStart).mockResolvedValue({
        v: 1,
        id: REQUEST,
        outcome: {
            status: "ok",
            value: { agent: "claude", pid: 4242, process: "ready", cols: 112, rows: 42 }
        }
    });
    vi.mocked(commands.terminalInput).mockResolvedValue(acknowledged());
    vi.mocked(commands.terminalResize).mockResolvedValue(acknowledged());
});

afterEach(() =>
{
    cleanup();
    // Cleared rather than reset: the terminal measures itself on the next
    // animation frame, and a frame that lands after a test has finished must
    // still find a command that answers with a promise.
    vi.clearAllMocks();
});

async function renderApp()
{
    const router = createMemoryRouter(routes, { initialEntries: [paths.today] });
    render(<RouterProvider router={router} />);

    await screen.findByText(WORKSPACE);
}

/** The channel the session gave the host, which is how the agent "prints". */
function printAgentOutput(bytes: Uint8Array)
{
    const call = vi.mocked(commands.terminalStart).mock.calls[0];

    if (call === undefined)
    {
        throw new Error("the terminal was never started");
    }

    const output = call[1] as unknown as {
        onmessage: ((message: { data_base64: string }) => void) | null;
    };

    output.onmessage?.({ data_base64: encodeBase64(bytes) });
}

function everythingWritten(terminal: TerminalDouble | undefined): number[]
{
    return (terminal?.written ?? []).reduce<number[]>((all, chunk) => [...all, ...chunk], []);
}

test("the session and its scrollback survive navigating every screen and back", async () =>
{
    const user = userEvent.setup();
    await renderApp();

    const terminal = registry.built[0] as TerminalDouble | undefined;
    const host = document.getElementById(TERMINAL_HOST_ID);
    expect(terminal).toBeDefined();
    expect(registry.built).toHaveLength(1);

    const printed = new Uint8Array([0x1b, 0x5b, 0x33, 0x31, 0x6d, 0x68, 0x69, 0xff]);
    printAgentOutput(printed);

    for (const section of ["Lab", "Strategies", "Today", "Lab"])
    {
        await user.click(screen.getByRole("link", { name: section }));

        expect(document.getElementById(TERMINAL_HOST_ID)).toBe(host);
        expect(registry.built).toHaveLength(1);
        expect(terminal?.disposed).toBe(false);
        expect(commands.terminalStart).toHaveBeenCalledTimes(1);
    }

    // Still there, and still the bytes the agent printed rather than a
    // re-rendered copy of them.
    expect(everythingWritten(terminal)).toEqual([...printed]);

    // And the live stream still reaches the same terminal afterwards.
    printAgentOutput(new Uint8Array([0x21]));
    expect(everythingWritten(terminal)).toEqual([...printed, 0x21]);

    // The other half of the same question: nothing the agent draws becomes
    // anything the app renders. The status bar reads the other channel, and
    // printing — including printing the word "Error" — does not move it.
    const statusBefore = screen.getByRole("contentinfo").textContent;
    printAgentOutput(new TextEncoder().encode("Error: everything is on fire\r\n"));
    expect(screen.getByRole("contentinfo").textContent).toBe(statusBefore);
});
