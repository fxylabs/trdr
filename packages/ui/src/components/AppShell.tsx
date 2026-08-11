import type { ReactNode } from "react";

import { TRDR_SCOPE_CLASS, classes } from "../contracts";

/** The three areas Phase 1/2 navigates between. */
export type WorkArea = "today" | "lab" | "strategies";

export type AppShellProps =
{
    /** Which area the centre view is showing. Read by the screen, not by CSS. */
    activeArea: WorkArea;
    /** The `<Sidebar/>`. A slot, so the app owns routing and this owns layout. */
    sidebar: ReactNode;
    /** The centre view. In the app this is the router's outlet. */
    children: ReactNode;
    /**
     * The agent rail's contents — the terminal, and nothing this package knows
     * about. It is a slot for the same reason the rail is a fixed region: the
     * PTY on the other end of it does not survive being remounted.
     */
    agentRail?: ReactNode;
    /** True for the whole of Phase 1/2. False leaves the third column empty. */
    agentVisible?: boolean;
    workspaceLabel?: string;
    agentRailLabel?: string;
};

/**
 * The window: 196px of navigation, a fluid work area, and a 420px agent rail.
 *
 * The columns are `.trdr-app-shell`'s, at the 1440×900 reference the contract
 * names. What this component adds is the guarantee behind them — the rail is a
 * region rendered in a fixed position, so navigating the centre view replaces
 * what is inside `children` and never touches the node the terminal attached to.
 * Putting the rail's contents in a slot rather than owning them is what makes
 * that a property of the layout instead of a promise about it.
 */
export function AppShell(props: AppShellProps)
{
    const {
        activeArea,
        sidebar,
        children,
        agentRail,
        agentVisible = true,
        workspaceLabel = "Workspace",
        agentRailLabel = "Agent terminal"
    } = props;

    return (
        <div
            className={classes("trdr-app-shell", TRDR_SCOPE_CLASS)}
            data-state="ready"
            data-area={activeArea}
            data-agent-visible={agentVisible ? "true" : "false"}
        >
            {sidebar}

            <main className="trdr-workspace" aria-label={workspaceLabel}>
                {children}
            </main>

            {agentVisible ? (
                <section className="trdr-agent-rail" aria-label={agentRailLabel}>
                    {agentRail}
                </section>
            ) : null}
        </div>
    );
}
