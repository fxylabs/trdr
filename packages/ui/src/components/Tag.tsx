import { classes } from "../contracts";

export type TagTone = "neutral" | "success" | "warning" | "info" | "focus";

export type TagProps =
{
    /** The state, spelled out. A tag whose meaning is only its colour says nothing. */
    label: string;
    tone?: TagTone;
};

/**
 * A state, reported.
 *
 * A tag is not a button and takes no handler — if something is clickable it is a
 * `Button`, and a tag that navigates is a tag that a keyboard cannot reach.
 */
export function Tag(props: TagProps)
{
    const { label, tone = "neutral" } = props;

    return (
        <span
            className={classes("trdr-tag", tone === "neutral" ? undefined : `trdr-tag--${tone}`)}
            data-state="rest"
        >
            {label}
        </span>
    );
}
