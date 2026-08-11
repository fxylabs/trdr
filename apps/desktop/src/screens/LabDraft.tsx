import { useNavigate } from "react-router";
import { Button, CoverageCard, EmptyState, Panel, RuleGrid, Tag, ViewLead } from "@trdr/ui";
import type { TagTone } from "@trdr/ui";

import type { DataCoverage, SectionState, SupportState } from "../bindings";
import { SCREEN, SUPPORT, SUPPORT_HEADLINE } from "../copy/ko";
import { share } from "../copy/format";
import { labDraftRequest, useModel } from "../ipc/model";
import { paths } from "../shell/routes";
import { Screen } from "./Screen";
import { SectionNote, coverageState } from "./sections";
import { ruleSections } from "./rules";

/**
 * The strategy being drafted, and whether trdr can run it (`LabDraftModel/v1`).
 *
 * The recipe is TopBar, ViewLead, RuleGrid, CoverageCard and a Button. The
 * headline is chosen by `support` and comes out of the localisation table —
 * section 11 forbids a headline from being written anywhere else, and there are
 * only four things it can say because `SupportState` has four values.
 *
 * The grid is `draft`, so it carries an edit affordance in the contract. It has
 * none here: editing a draft is outside this milestone, and an affordance that
 * does nothing is worse than an absent one.
 */
export function LabDraft()
{
    const navigate = useNavigate();
    const state = useModel(labDraftRequest());

    return (
        <Screen title={SCREEN.labDraft.title} context={SCREEN.labDraft.context} state={state}>
            {(model) => (
                <>
                    <ViewLead
                        pill={model.name}
                        eyebrow={SCREEN.labDraft.period(model.period_start, model.period_end)}
                        headline={SUPPORT_HEADLINE[model.support]}
                        status={<Tag tone={supportTone(model.support)} label={SUPPORT[model.support]} />}
                    />

                    <Panel title={SCREEN.labDraft.rules} caption={model.source_path}>
                        <RuleGrid {...ruleSections(model.rules)} state={model.rules.section} />
                    </Panel>

                    <SectionNote state={model.coverage_state} />
                    <Coverage coverage={model.coverage} state={model.coverage_state} />

                    <Button
                        variant="primary"
                        label={SCREEN.labDraft.seeResult}
                        onActivate={() => navigate(paths.labResult)}
                    />
                </>
            )}
        </Screen>
    );
}

/** Supported is the only state that is not a reason the run will not happen. */
function supportTone(support: SupportState): TagTone
{
    return support === "supported" ? "success" : "warning";
}

/** One card per source, or the reason there is no card at all. */
function Coverage(props: { readonly coverage: readonly DataCoverage[]; readonly state: SectionState })
{
    const { coverage, state } = props;

    if (coverage.length === 0)
    {
        return (
            <Panel>
                <EmptyState
                    title={SCREEN.labDraft.emptyCoverage}
                    explanation={SCREEN.labDraft.emptyCoverageWhy}
                />
            </Panel>
        );
    }

    return (
        <>
            {coverage.map((entry) => (
                <CoverageCard
                    key={entry.source}
                    source={entry.source}
                    days={entry.total_days}
                    covered={entry.covered_days}
                    total={entry.total_days}
                    summary={coverageSummary(entry)}
                    state={coverageState(state)}
                />
            ))}
        </>
    );
}

/**
 * The coverage in words, naming which days are missing rather than how many.
 *
 * A run refused for missing data is a run someone has to go and collect
 * something for, and "68일 결측" does not say what to collect. The gap ranges
 * are what does.
 */
function coverageSummary(entry: DataCoverage): string
{
    const covered = SCREEN.labDraft.coverageSummary(
        entry.covered_days,
        entry.total_days,
        share(entry.covered_days, entry.total_days)
    );

    if (entry.gaps.length === 0)
    {
        return `${covered} · ${SCREEN.labDraft.coverageNoGaps}`;
    }

    const ranges = entry.gaps.map((gap) => `${gap.from} ~ ${gap.to}`).join(", ");

    return `${covered} · ${SCREEN.labDraft.coverageGaps(ranges)}`;
}
