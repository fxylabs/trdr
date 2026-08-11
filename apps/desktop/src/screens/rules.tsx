import type { RuleGridProps } from "@trdr/ui";

import type { Rule, StrategyRules } from "../bindings";
import { SCREEN } from "../copy/ko";

/**
 * A strategy's normalised rules, in the four sections the grid draws.
 *
 * Three screens show the same four sections — the draft, the result that froze
 * them, and the registration that is being validated against them — so the
 * mapping is written once. The headings are the screen's copy rather than the
 * kit's, because `RuleGrid` takes a heading per section precisely so that the
 * package holds no Korean.
 *
 * A rule's label and value are both text by the time they arrive: normalisation
 * happened in the query service, and a threshold, a universe and a window do not
 * share a type. Joining them here is presentation and nothing more.
 */
function lines(rules: readonly Rule[]): readonly string[]
{
    return rules.map((rule) => SCREEN.labDraft.rule(rule.label, rule.value));
}

/** The four sections, ready to spread into `RuleGrid`. */
export function ruleSections(
    rules: StrategyRules
): Pick<RuleGridProps, "universe" | "entry" | "exitAndCost" | "validationWindow">
{
    return {
        universe: { heading: SCREEN.labDraft.sectionUniverse, rules: lines(rules.universe) },
        entry: { heading: SCREEN.labDraft.sectionEntry, rules: lines(rules.entry) },
        exitAndCost: {
            heading: SCREEN.labDraft.sectionExitAndCost,
            rules: lines(rules.exit_and_cost)
        },
        validationWindow: {
            heading: SCREEN.labDraft.sectionValidationWindow,
            rules: lines(rules.validation_window)
        }
    };
}
