import { useId } from "react";
import type { ChangeEvent, KeyboardEvent } from "react";

export type FieldProps =
{
    /** Always visible, always tied to the control. A placeholder is not a label. */
    label: string;
    value: string;
    invalid?: boolean;
    disabled?: boolean;
    hint?: string;
    /** The reason it is invalid. Its presence marks the control invalid too. */
    error?: string;
    inputMode?: "text" | "numeric" | "decimal";
    onChange?: (value: string) => void;
    /** Blur, or Enter. The value the user meant rather than the one mid-typing. */
    onCommit?: (value: string) => void;
};

/**
 * A user-authored parameter.
 *
 * For things the user writes — a strategy name, a window, a threshold. Agent
 * output is not a value in a field; it is characters in the terminal, and it
 * stays there.
 */
export function Field(props: FieldProps)
{
    const { label, value, disabled = false, hint, error, inputMode, onChange, onCommit } = props;
    const invalid = props.invalid === true || error !== undefined;
    const id = useId();
    const described = [hint === undefined ? undefined : `${id}-hint`, error === undefined ? undefined : `${id}-error`]
        .filter((token): token is string => token !== undefined)
        .join(" ");

    return (
        <div className="trdr-field-group">
            <label className="trdr-field-label" htmlFor={id}>
                {label}
            </label>

            <input
                id={id}
                className="trdr-field"
                data-state={disabled ? "disabled" : invalid ? "invalid" : "rest"}
                value={value}
                disabled={disabled}
                aria-invalid={invalid ? "true" : undefined}
                aria-describedby={described === "" ? undefined : described}
                {...(inputMode === undefined ? {} : { inputMode })}
                onChange={(event: ChangeEvent<HTMLInputElement>) => onChange?.(event.target.value)}
                onBlur={(event) => onCommit?.(event.target.value)}
                onKeyDown={(event: KeyboardEvent<HTMLInputElement>) =>
                {
                    if (event.key === "Enter")
                    {
                        onCommit?.(event.currentTarget.value);
                    }
                }}
            />

            {hint === undefined ? null : (
                <span className="trdr-field-hint" id={`${id}-hint`}>
                    {hint}
                </span>
            )}

            {error === undefined ? null : (
                <span className="trdr-field-error" id={`${id}-error`}>
                    {error}
                </span>
            )}
        </div>
    );
}
