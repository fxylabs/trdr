import type { ReactNode } from "react";

import { NavigationItem } from "./NavigationItem";

export type NavigationItemModel =
{
    id: string;
    label: string;
    href?: string;
    current?: boolean;
};

export type SidebarProps =
{
    /** The wordmark. The lime dot after it is drawn by `.trdr-brand`. */
    brand?: string;
    /** Today, Lab, Strategies. Phase 1/2 has no fourth. */
    items: readonly NavigationItemModel[];
    onNavigate?: (id: string) => void;
    /** A `<BrokerConnection/>`. It sits at the bottom by way of `margin-top: auto`. */
    connection?: ReactNode;
    /** The nav landmark's name. */
    label?: string;
};

/**
 * The left column: brand, navigation, broker connection.
 *
 * There is no market discovery, news or watchlist entry here, and that is the
 * contract's rule rather than an omission — Phase 1/2 navigates between three
 * places and the sidebar says so by having three places in it.
 */
export function Sidebar(props: SidebarProps)
{
    const { brand, items, onNavigate, connection, label = "Sections" } = props;

    return (
        <div className="trdr-sidebar" data-state="ready">
            {brand === undefined ? null : <div className="trdr-brand">{brand}</div>}

            <nav className="trdr-nav" aria-label={label}>
                {items.map((item) => (
                    <NavigationItem
                        key={item.id}
                        label={item.label}
                        {...(item.href === undefined ? {} : { href: item.href })}
                        {...(item.current === undefined ? {} : { current: item.current })}
                        {...(onNavigate === undefined ? {} : { onActivate: () => onNavigate(item.id) })}
                    />
                ))}
            </nav>

            {connection}
        </div>
    );
}
