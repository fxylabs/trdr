import { Outlet, useLocation, useNavigate } from "react-router";
import { AppShell as KitAppShell, BrokerConnection, Sidebar, Tag } from "@trdr/ui";
import type { NavigationItemModel, WorkArea } from "@trdr/ui";

import type { TodayModel } from "../bindings";
import { BROKER_CONNECTION, ORIGIN, SHELL } from "../copy/ko";
import { instant } from "../copy/format";
import { todayRequest, useModel } from "../ipc/model";
import type { ModelState } from "../ipc/model";
import { areaOf, navigation, paths } from "./routes";
import { StatusLine } from "./StatusLine";
import { TerminalHost } from "./TerminalHost";

/**
 * The frame every screen is shown inside.
 *
 * This is a layout route, so React Router mounts it once and then only replaces
 * what `<Outlet/>` renders. Three things depend on that:
 *
 * - The agent rail keeps its DOM node across navigation. The kit's `AppShell`
 *   renders the rail as a fixed region and takes its contents as a slot, and
 *   `TerminalHost` is passed into that slot from here — the same position on
 *   every render, under the same parent, so reconciliation keeps the node. See
 *   `TerminalHost` for why that is not a detail, and `AppShell.test.tsx` for
 *   the test that holds it.
 * - The navigation is rendered once rather than rebuilt per screen.
 * - The two things the window knows — which workspace it opened, and whether
 *   what it is showing is real — are asked once here rather than once per
 *   screen.
 *
 * # Why the shell asks for Today's model
 *
 * The sidebar's broker connection is part of `Sidebar`'s anatomy in the visual
 * contract, and the state behind it lives on `TodayModel`'s account. So the
 * shell needs that model whatever screen is open. Having it, the shell also
 * derives the window-level synthetic statement from the same `origin` field
 * rather than from a constant — which means the assertion that these are not
 * real numbers is on screen even before a screen has finished loading, and is
 * not something any screen can forget to make.
 */
export function AppShell()
{
    const location = useLocation();
    const navigate = useNavigate();
    const today = useModel(todayRequest());
    const area = areaOf(location.pathname);

    return (
        <KitAppShell
            activeArea={area}
            workspaceLabel={SHELL.workspace}
            agentRailLabel={SHELL.agentRail}
            sidebar={
                <Sidebar
                    brand={SHELL.brand}
                    label={SHELL.navigation}
                    items={items(area)}
                    onNavigate={(id) => navigate(pathOf(id))}
                    connection={
                        <>
                            <Origin state={today} />
                            <Connection state={today} />
                            <StatusLine />
                        </>
                    }
                />
            }
            agentRail={<TerminalHost />}
        >
            <Outlet />
        </KitAppShell>
    );
}

/** The three destinations, with the one the user is in marked. */
function items(area: WorkArea): readonly NavigationItemModel[]
{
    return navigation.map((entry) => ({
        id: entry.area,
        label: entry.label,
        href: `#${entry.path}`,
        current: entry.area === area
    }));
}

/** Where a navigation id goes. The route table is the only place that decides. */
function pathOf(id: string): string
{
    return navigation.find((entry) => entry.area === id)?.path ?? paths.today;
}

/**
 * What the window is showing, in two words, wherever the user is.
 *
 * Read from the model's `origin` and nothing else, so it stops saying
 * `합성 데이터` on the day the data stops being synthetic. The sidebar never
 * scrolls, which is what makes this the persistent half of milestone M2's
 * requirement; the sentence-length half is on every screen, above its content.
 */
function Origin(props: { readonly state: ModelState<TodayModel> })
{
    const { state } = props;

    if (state.status !== "ready")
    {
        return null;
    }

    const origin = state.model.origin;

    return <Tag tone={origin === "synthetic" ? "warning" : "neutral"} label={ORIGIN[origin].label} />;
}

/**
 * The broker link at the foot of the sidebar.
 *
 * Green means the system is connected and never that an investment is doing
 * well, which is why the state comes from the account's `connection` field and
 * the word beside it comes from the localisation table. A window that has not
 * heard back yet says it is syncing; one that was refused says the attempt
 * failed, rather than showing a connected dot over a model it never received.
 */
function Connection(props: { readonly state: ModelState<TodayModel> })
{
    const { state } = props;
    const account = state.status === "ready" ? state.model.account : undefined;
    const connection = account?.connection ?? (state.status === "loading" ? "syncing" : "error");

    return (
        <BrokerConnection
            broker={SHELL.broker}
            state={connection}
            stateLabel={BROKER_CONNECTION[connection]}
            authority={SHELL.brokerAuthority}
            {...(account === undefined ? {} : { lastSync: instant(account.as_of) })}
        />
    );
}
