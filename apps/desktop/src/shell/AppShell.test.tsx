import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { RouterProvider, createMemoryRouter } from "react-router";
import { afterEach, beforeEach, expect, test, vi } from "vitest";

import type { BootstrapResponse_Serialize, PingResponse_Serialize } from "../bindings";
import { commands } from "../bindings";
import { paths, routes } from "./routes";
import { TERMINAL_HOST_ID } from "./TerminalHost";

/**
 * The host, replaced.
 *
 * Everything below `commands` is Tauri's `invoke`, which needs a WebView with a
 * host behind it and has none here. What is being tested is what the shell does
 * with an answer, so the answer is supplied; that the real host gives one is
 * `src-tauri/tests/capability.rs`'s job, and that the two meet is the live smoke
 * run's.
 */
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

/**
 * The terminal, without a terminal.
 *
 * The agent rail inside the host is a real component now, and mounting it here
 * would drag in xterm's renderer and a Tauri channel — neither of which exists
 * in a jsdom document, and neither of which these tests are about. What is
 * being tested is the shell: three screens, a status line, and a host node that
 * survives navigation. `terminal/persistence.test.tsx` is where the rail's own
 * behaviour is held, against the same route table.
 */
vi.mock("@xterm/xterm", () => ({
    Terminal: class
    {
        public cols = 80;

        public rows = 24;

        public element: HTMLElement | null = null;

        public loadAddon(): void {}

        public open(host: HTMLElement): void
        {
            this.element = document.createElement("div");
            host.append(this.element);
        }

        public write(): void {}

        public onData(): { dispose: () => void }
        {
            return { dispose: () => undefined };
        }

        public onBinary(): { dispose: () => void }
        {
            return { dispose: () => undefined };
        }

        public dispose(): void {}
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

const WORKSPACE = "01KZNP0GQ3X8ARK9DQ489Z7WJ8";
const REQUEST = "01KZNNR5X818P3J6ENYKSADP8W";

const pong: PingResponse_Serialize = {
    v: 1,
    id: REQUEST,
    outcome: { status: "ok", value: { protocol_version: 1 } }
};

const bootstrap: BootstrapResponse_Serialize = {
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
};

const acknowledged = {
    v: 1 as const,
    id: REQUEST,
    outcome: { status: "ok" as const, value: { process: "ready" as const } }
};

beforeEach(() =>
{
    vi.mocked(commands.ping).mockResolvedValue(pong);
    vi.mocked(commands.bootstrapGet).mockResolvedValue(bootstrap);
    vi.mocked(commands.terminalStart).mockResolvedValue({
        v: 1,
        id: REQUEST,
        outcome: {
            status: "ok",
            value: { agent: "claude", pid: 4242, process: "ready", cols: 80, rows: 24 }
        }
    });
    vi.mocked(commands.terminalInput).mockResolvedValue(acknowledged);
    vi.mocked(commands.terminalResize).mockResolvedValue(acknowledged);
    vi.mocked(commands.terminalRestart).mockResolvedValue({
        v: 1,
        id: REQUEST,
        outcome: {
            status: "ok",
            value: { agent: "claude", pid: 4243, process: "ready", cols: 80, rows: 24 }
        }
    });
});

afterEach(() =>
{
    document.body.innerHTML = "";
    vi.resetAllMocks();
});

/**
 * The shell, mounted through the same route table the app uses.
 *
 * A memory router rather than the hash router, because the assertion is about
 * what survives a navigation and not about how the URL is written down.
 *
 * Mounting waits for the host round trip to settle. Not for the assertion's
 * sake — the tests below are about navigation — but because a state update
 * landing after a test has finished belongs to no test at all, and is the kind
 * of thing that fails once a week on someone else's machine.
 */
async function renderShell()
{
    const router = createMemoryRouter(routes, { initialEntries: [paths.today] });
    const view = render(<RouterProvider router={router} />);

    await screen.findByText(WORKSPACE);

    return view;
}

/** The one id a command was called with, or a failure saying it was not. */
function theIdItWasCalledWith(calls: readonly (readonly [string])[]): string
{
    const [call] = calls;

    if (call === undefined)
    {
        throw new Error("the command was never called");
    }

    expect(calls).toHaveLength(1);

    return call[0];
}

function terminalHost(): HTMLElement
{
    const host = document.getElementById(TERMINAL_HOST_ID);

    if (host === null)
    {
        throw new Error("the terminal host is not in the document");
    }

    return host;
}

test("the three sections are reachable", async () =>
{
    const user = userEvent.setup();
    await renderShell();

    expect(screen.getByRole("heading", { name: "Today" })).toBeDefined();

    await user.click(screen.getByRole("link", { name: "Lab" }));
    expect(screen.getByRole("heading", { name: "Lab" })).toBeDefined();

    await user.click(screen.getByRole("link", { name: "Strategies" }));
    expect(screen.getByRole("heading", { name: "Strategies" })).toBeDefined();
});

/**
 * The one thing section 14 item 3 asks of the terminal host.
 *
 * What is asserted is node identity, not presence. A host that unmounted and
 * remounted would still be findable by id after every navigation and would still
 * be useless: whatever the terminal attached to the old node — the canvas, the
 * scrollback, the process on the other end of it — went away with it.
 *
 * The sentinel is that attachment, in miniature. Something outside React writes
 * into the node, exactly as xterm will, and then the test navigates through every
 * screen and back. If the sentinel is still there, React never replaced the node.
 */
test("the terminal host keeps its node across every navigation", async () =>
{
    const user = userEvent.setup();
    await renderShell();

    const host = terminalHost();
    const sentinel = document.createElement("span");
    sentinel.textContent = "attached out of band, the way a terminal attaches";
    host.append(sentinel);

    for (const section of ["Lab", "Strategies", "Today", "Lab"])
    {
        await user.click(screen.getByRole("link", { name: section }));

        expect(terminalHost()).toBe(host);
        expect(host.contains(sentinel)).toBe(true);
    }

    expect(host.isConnected).toBe(true);
});

/**
 * What is inside the host is the agent rail, and only the agent rail.
 *
 * The host used to be empty, and asserting that it stayed empty was how this
 * file kept a screen from rendering into it. The rail lives there now, so the
 * check is the same one stated against what is there: one child, and it is the
 * terminal shell. A screen that started rendering into this node would take its
 * contents with it on the next navigation — the failure the test above is
 * about, arriving by a different door.
 */
test("the terminal host holds the agent rail and belongs to the layout", async () =>
{
    await renderShell();

    const host = terminalHost();

    expect(host.childElementCount).toBe(1);
    expect(host.firstElementChild?.className).toBe("trdr-terminal-shell");
    expect(screen.getByRole("region", { name: "Agent terminal" })).toBeDefined();
});

/**
 * Section 14's stop condition, from this side: the shell asks the host and puts
 * what came back on the screen.
 *
 * The workspace id is asserted rather than merely present, because a status line
 * rendering a constant would look identical to one rendering a response.
 */
test("the shell shows the workspace the host answered with", async () =>
{
    await renderShell();

    const status = screen.getByRole("status");

    expect(status.textContent).toContain(WORKSPACE);
    expect(status.textContent).toContain("9.9.9");
    expect(commands.ping).toHaveBeenCalledTimes(1);
    expect(commands.bootstrapGet).toHaveBeenCalledTimes(1);
});

/**
 * The two commands are two requests, and the id is what says so. Reusing one
 * would be the screen claiming they were the same request — which section 9.2
 * gives a meaning to: the same terminal result rather than a second execution.
 */
test("each command carries an id the host would accept, and its own", async () =>
{
    await renderShell();

    const ids = [
        theIdItWasCalledWith(vi.mocked(commands.ping).mock.calls),
        theIdItWasCalledWith(vi.mocked(commands.bootstrapGet).mock.calls)
    ];

    for (const id of ids)
    {
        expect(id).toMatch(/^[0-7][0-9A-HJKMNP-TV-Z]{25}$/);
    }

    expect(ids[0]).not.toBe(ids[1]);
});

/**
 * A host that is not there says so. The failure this rules out is the quiet
 * one: a status line that stays on its loading text forever, which looks like a
 * slow app rather than a broken bridge.
 */
test("a host that cannot be reached is named rather than left loading", async () =>
{
    vi.mocked(commands.ping).mockRejectedValue(new Error("there is no host here"));

    const router = createMemoryRouter(routes, { initialEntries: [paths.today] });
    render(<RouterProvider router={router} />);

    const status = await screen.findByText("APP_NOT_RUNNING");

    expect(status).toBeDefined();
    expect(commands.bootstrapGet).not.toHaveBeenCalled();
});

/**
 * A refusal is an envelope, not a rejected promise (see `commands.rs`), and the
 * shell has to read the code out of it rather than treat the response as a
 * success because it arrived.
 */
test("a refused command is read out of the envelope it came in", async () =>
{
    vi.mocked(commands.bootstrapGet).mockResolvedValue({
        v: 1,
        id: REQUEST,
        outcome: {
            status: "error",
            value: { v: 1, code: "DB_BUSY", retryability: { kind: "no" } }
        }
    });

    const router = createMemoryRouter(routes, { initialEntries: [paths.today] });
    render(<RouterProvider router={router} />);

    expect(await screen.findByText("DB_BUSY")).toBeDefined();
});
