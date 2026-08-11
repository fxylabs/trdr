export type RuleGridState = "draft" | "frozen";

/** The four sections, in the order the grid lays them out. */
export type RuleGridSectionId = "universe" | "entry" | "exitAndCost" | "validationWindow";

export type RuleSection =
{
    /** `대상과 순위`. The kit holds no copy; the screen names its own sections. */
    heading: string;
    /** One normalized rule per line. `MA20 > MA60`, `최대 10종목`. */
    rules: readonly string[];
};

export type RuleGridProps =
{
    universe: RuleSection;
    entry: RuleSection;
    exitAndCost: RuleSection;
    validationWindow: RuleSection;
    state?: RuleGridState;
    /** Draft only. A frozen grid is given no edit affordance at all. */
    onEdit?: (section: RuleGridSectionId) => void;
    editLabel?: string;
};

/**
 * A strategy's rules, normalized, in the four groups it is registered as.
 *
 * The state is the discipline. A draft can be edited; a frozen grid is what a
 * paper validation is running against, and the contract's rule is that it shows
 * no way to edit it — not a disabled control, not an edit that fails, nothing.
 * Rules that could be changed mid-validation would make the result meaningless,
 * so the affordance is absent rather than refused.
 */
export function RuleGrid(props: RuleGridProps)
{
    const { state = "draft", onEdit, editLabel } = props;
    const sections: readonly (readonly [RuleGridSectionId, RuleSection])[] = [
        ["universe", props.universe],
        ["entry", props.entry],
        ["exitAndCost", props.exitAndCost],
        ["validationWindow", props.validationWindow]
    ];

    return (
        <div className="trdr-rule-grid" data-state={state}>
            {sections.map(([id, section]) => (
                <div key={id} className="trdr-rule-section" data-section={id}>
                    <h4>
                        {section.heading}
                        {state === "draft" && onEdit !== undefined && editLabel !== undefined ? (
                            <>
                                {" "}
                                <button className="trdr-inline-action" type="button" onClick={() => onEdit(id)}>
                                    {editLabel}
                                </button>
                            </>
                        ) : null}
                    </h4>

                    {section.rules.map((rule) => (
                        <div key={rule} className="trdr-rule-value">
                            {rule}
                        </div>
                    ))}
                </div>
            ))}
        </div>
    );
}
