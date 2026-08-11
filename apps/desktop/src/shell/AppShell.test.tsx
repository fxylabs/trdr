import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { RouterProvider, createMemoryRouter } from "react-router";
import { afterEach, expect, test } from "vitest";

import { paths, routes } from "./routes";
import { TERMINAL_HOST_ID } from "./TerminalHost";

/**
 * The shell, mounted through the same route table the app uses.
 *
 * A memory router rather than the hash router, because the assertion is about
 * what survives a navigation and not about how the URL is written down.
 */
function renderShell()
{
    const router = createMemoryRouter(routes, { initialEntries: [paths.today] });
    return render(<RouterProvider router={router} />);
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

afterEach(() =>
{
    document.body.innerHTML = "";
});

test("the three sections are reachable", async () =>
{
    const user = userEvent.setup();
    renderShell();

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
    renderShell();

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
 * The host is empty until the track that owns the PTY fills it. A screen that
 * started rendering into it would take the node's contents with it on the next
 * navigation, which is the failure the test above is about, arriving by a
 * different door.
 */
test("the terminal host starts empty and belongs to the layout", () =>
{
    renderShell();

    expect(terminalHost().childElementCount).toBe(0);
    expect(screen.getByRole("region", { name: "Agent terminal" })).toBeDefined();
});
