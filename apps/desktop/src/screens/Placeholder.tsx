/** What a screen needs to say while it is still a placeholder. */
type PlaceholderProps = {
    /** The screen's name, as the navigation spells it. */
    readonly title: string;
    /** The section 9.1 command that will supply this screen's model. */
    readonly command: string;
};

/**
 * A screen that exists so the shell can be navigated, and says so.
 *
 * Naming the command each screen is waiting on keeps this honest: it is the
 * difference between a placeholder and an empty state, and it means nobody has
 * to read the route table to find out what is missing.
 */
export function Placeholder({ title, command }: PlaceholderProps)
{
    return (
        <article className="placeholder">
            <h1 className="placeholder__title">{title}</h1>
            <p className="placeholder__note">
                Waiting on <code>{command}</code>.
            </p>
        </article>
    );
}
