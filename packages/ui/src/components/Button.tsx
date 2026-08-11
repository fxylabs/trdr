import { classes } from "../contracts";

export type ButtonVariant = "default" | "primary" | "quiet";

export type ButtonProps =
{
    label: string;
    variant?: ButtonVariant;
    disabled?: boolean;
    /** `submit` only inside a form that means it. Everything else is a button. */
    type?: "button" | "submit";
    onActivate?: () => void;
};

/**
 * A native button, and always a native button.
 *
 * The primary variant is graphite with a lime dot on it. The dot is the whole of
 * the lime: a filled lime button would make the accent a fill system, and the
 * accent's job in this UI is to point at one thing — where you are, and what is
 * about to happen. The dot is `.trdr-button--primary`'s pseudo-element, so a
 * variant cannot be given the colour without the shape.
 *
 * Disabled is the `disabled` attribute rather than a class, which is what the
 * contract means by programmatic: the control is unreachable by keyboard as well
 * as dimmed.
 */
export function Button(props: ButtonProps)
{
    const { label, variant = "default", disabled = false, type = "button", onActivate } = props;

    return (
        <button
            className={classes("trdr-button", variant === "default" ? undefined : `trdr-button--${variant}`)}
            data-state={disabled ? "disabled" : "rest"}
            type={type}
            disabled={disabled}
            onClick={onActivate}
        >
            {label}
        </button>
    );
}
