import type { MouseEvent } from "react";

export type NavigationItemProps =
{
    label: string;
    /** The one item that is where the user is. Lime means this and nothing else. */
    current?: boolean;
    /** Given, the item is a link; omitted, it is a button. Both are contract-legal. */
    href?: string;
    onActivate?: () => void;
};

/**
 * One destination in the sidebar.
 *
 * The state dot and the lime marker beside a current item are `.trdr-nav-item`'s
 * pseudo-elements, and they hang off `aria-current="page"` rather than off a
 * modifier class — so the thing a screen reader is told and the thing the eye is
 * shown are the same attribute, and cannot come apart.
 */
export function NavigationItem(props: NavigationItemProps)
{
    const { label, current = false, href, onActivate } = props;

    const shared =
    {
        className: "trdr-nav-item",
        "data-state": current ? "current" : "rest",
        "aria-current": current ? ("page" as const) : undefined
    };

    if (href !== undefined)
    {
        return (
            <a
                {...shared}
                href={href}
                onClick={(event: MouseEvent<HTMLAnchorElement>) =>
                {
                    if (onActivate !== undefined)
                    {
                        event.preventDefault();
                        onActivate();
                    }
                }}
            >
                {label}
            </a>
        );
    }

    return (
        <button {...shared} type="button" onClick={onActivate}>
            {label}
        </button>
    );
}
