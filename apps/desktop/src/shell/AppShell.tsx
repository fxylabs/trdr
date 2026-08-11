import { NavLink, Outlet } from "react-router";

import { navigation } from "./routes";
import { StatusLine } from "./StatusLine";
import { TerminalHost } from "./TerminalHost";

/**
 * The frame every screen is shown inside.
 *
 * This is a layout route, so React Router mounts it once and then only replaces
 * what `<Outlet/>` renders. Two things depend on that and nothing else:
 *
 * - The terminal host below the outlet keeps its DOM node across navigation. See
 *   `TerminalHost` for why that is not a detail, and `AppShell.test.tsx` for the
 *   test that holds it.
 * - The navigation is rendered once rather than rebuilt per screen.
 * - The status line asks the host once, on mount, rather than once per screen.
 *   It is the only thing in the shell that talks to Rust, and it is here because
 *   this is the component whose lifetime is the window's.
 *
 * The layout is placeholder. Colours, spacing, and type come from the UI kit in
 * `design/ui-kit`, and the work of actually applying it is a later unit; what is
 * here is the minimum that makes the three regions distinguishable on screen.
 */
export function AppShell()
{
    return (
        <div className="shell">
            <nav className="shell__nav" aria-label="Sections">
                {navigation.map(({ path, label }) => (
                    <NavLink
                        key={path}
                        to={path}
                        end={path === "/"}
                        className={({ isActive }) =>
                            isActive ? "shell__link shell__link--active" : "shell__link"
                        }
                    >
                        {label}
                    </NavLink>
                ))}
            </nav>

            <main className="shell__screen">
                <Outlet />
            </main>

            <TerminalHost />

            <StatusLine />
        </div>
    );
}
