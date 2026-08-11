export type BackTarget =
{
    /**
     * The destination, named. `전략 목록`, not `뒤로`. The contract asks the back
     * control to say where it goes, which is also the only way the control has
     * an accessible name worth reading.
     */
    label: string;
    onActivate: () => void;
};

export type TopBarProps =
{
    /** The view's `h1`. One per view. */
    title: string;
    context?: string;
    metadata?: string;
    /** Initials. Decorative — the account is not identified by this tile. */
    avatar?: string;
    /** Present on every detail view, by contract. */
    back?: BackTarget;
};

/**
 * The bar across the top of the work area.
 *
 * `title` is the view's `h1` and there is no second one: `ViewLead`'s headline
 * is an `h2` that follows it, so a screen's heading order is the order it reads.
 */
export function TopBar(props: TopBarProps)
{
    const { title, context, metadata, avatar, back } = props;

    return (
        <header className="trdr-topbar" data-state="ready">
            {back === undefined ? null : (
                <button className="trdr-back-button" type="button" onClick={back.onActivate}>
                    {`← ${back.label}`}
                </button>
            )}

            <h1>{title}</h1>

            {context === undefined ? null : <span className="trdr-topbar-context">{context}</span>}
            {metadata === undefined ? null : <span className="trdr-topbar-meta">{metadata}</span>}

            {avatar === undefined ? null : (
                <span className="trdr-avatar" aria-hidden="true">
                    {avatar}
                </span>
            )}
        </header>
    );
}
