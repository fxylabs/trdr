/**
 * The terminal, without a terminal.
 *
 * Every test that renders the route table gets the agent rail with it, because
 * the rail is a child of the layout route and that is the whole point of where
 * it sits. Rendering it for real would need xterm's renderer, which wants a
 * canvas jsdom does not implement, and a Tauri `Channel`, which reaches for the
 * WebView's IPC the moment it is constructed. Neither is what a shell test or a
 * screen test is about.
 *
 * So the three modules behind the rail are replaced, and they are replaced from
 * here rather than in each test file: three files needed the same sixty lines,
 * and three copies of a stub are three chances for one of them to drift into
 * proving something the others do not.
 *
 * These are for tests that render *past* the rail. The rail's own behaviour —
 * the session, the bytes, the process states, the scrollback surviving
 * navigation — is held in `persistence.test.tsx` and the tests beside it, which
 * build their own doubles because there they are the subject rather than the
 * scenery.
 *
 * `vi.mock` is hoisted above the imports of the file that calls it, so a test
 * passes these as factories rather than importing an installed mock:
 *
 *     vi.mock("@xterm/xterm", () => xtermModule());
 */

/** A terminal that mounts a node and swallows everything else. */
export function xtermModule()
{
    return {
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
    };
}

/** The fit addon, doing nothing. */
export function fitAddonModule()
{
    return {
        FitAddon: class
        {
            public fit(): void {}
        }
    };
}

/** xterm's stylesheet, which a test has no use for. */
export function xtermStylesModule(): Record<string, never>
{
    return {};
}

/**
 * Tauri's core, with a channel that carries nothing.
 *
 * The real `Channel` registers a callback with the WebView's IPC in its
 * constructor, and fails outside one with a message about `transformCallback`
 * that names nothing a reader would connect to a terminal.
 */
export function tauriCoreModule()
{
    return {
        Channel: class
        {
            public onmessage: ((message: unknown) => void) | null = null;
        }
    };
}
