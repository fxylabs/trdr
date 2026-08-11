import { useNavigate } from "react-router";
import { Button, EmptyState, InlineFeedback, Panel, StrategyCard, Tag, ViewLead } from "@trdr/ui";
import type { StrategyCardState, StrategySummary as CardSummary, TagTone } from "@trdr/ui";

import type { PaperValidationState, SectionState, StrategySummary } from "../bindings";
import { PAPER_VALIDATION, SCREEN } from "../copy/ko";
import { share, signedPercent } from "../copy/format";
import { strategiesRequest, useModel } from "../ipc/model";
import { paths, strategyPath } from "../shell/routes";
import { Screen } from "./Screen";
import { SectionNote } from "./sections";

/**
 * Every strategy the person has registered (`StrategiesModel/v1`).
 *
 * The recipe is TopBar, ViewLead, StrategyCard, Tag and InlineFeedback. The
 * card's own rule — performance never outranks observation progress and rule
 * deviations — is the kit's to enforce and this screen's not to undo: nothing
 * above the list carries a return, the lead talks about observation, and the
 * card is handed its three statistics in the order the contract fixes.
 */
export function Strategies()
{
    const state = useModel(strategiesRequest());

    return (
        <Screen title={SCREEN.strategies.title} context={SCREEN.strategies.context} state={state}>
            {(model) => (
                <>
                    <ViewLead
                        headline={SCREEN.strategies.headline}
                        description={SCREEN.strategies.description}
                    />

                    <Deviations strategies={model.strategies} />
                    <SectionNote state={model.state} />
                    <List state={model.state} strategies={model.strategies} />
                </>
            )}
        </Screen>
    );
}

/**
 * The list, or the reason there is none.
 *
 * `empty` and `error` are two different facts and get two different surfaces: a
 * person who has not registered anything is told what to do next, and a list
 * that could not be built is an alert (`SectionNote`, above) with no invitation
 * attached. Collapsing them would tell a new user that something broke.
 */
function List(props: {
    readonly state: SectionState;
    readonly strategies: readonly StrategySummary[];
})
{
    const { state, strategies } = props;
    const navigate = useNavigate();

    if (state === "error")
    {
        return null;
    }

    if (strategies.length === 0)
    {
        return (
            <Panel>
                <EmptyState
                    title={SCREEN.strategies.empty}
                    explanation={SCREEN.strategies.emptyWhy}
                    action={
                        <Button
                            variant="primary"
                            label={SCREEN.today.openLab}
                            onActivate={() => navigate(paths.lab)}
                        />
                    }
                />
            </Panel>
        );
    }

    return (
        <div className="trdr-strategy-list">
            {strategies.map((summary) => (
                <StrategyCard
                    key={summary.strategy}
                    strategy={card(summary)}
                    state={cardState(summary.validation)}
                    href={`#${strategyPath(summary.strategy)}`}
                    onOpen={(id) => navigate(strategyPath(id))}
                />
            ))}
        </div>
    );
}

/** How many times a registered rule was not followed, across the whole list. */
function Deviations(props: { readonly strategies: readonly StrategySummary[] })
{
    const total = props.strategies.reduce((sum, summary) => sum + summary.deviations, 0);

    if (total === 0)
    {
        return null;
    }

    return <InlineFeedback tone="warning" message={SCREEN.strategies.deviationsWarning(total)} />;
}

/**
 * One registration as the card renders it.
 *
 * The order of the three statistics is the contract's claim about what matters,
 * and it is fixed by `StrategySummary`'s field order rather than by this
 * function: observation first, the paper return second and uncoloured, what the
 * run is doing now third.
 */
function card(summary: StrategySummary): CardSummary
{
    const { elapsed_days: elapsed, total_days: total } = summary.observation;

    return {
        id: summary.strategy,
        name: summary.name,
        description: summary.description,
        status: <Tag tone={statusTone(summary.validation)} label={statusLabel(summary)} />,
        observation: {
            label: SCREEN.strategies.observation,
            value: SCREEN.strategies.observationValue(elapsed, total),
            note: SCREEN.strategies.observationNote(share(elapsed, total))
        },
        paperReturn: {
            label: SCREEN.strategies.paperReturn,
            value: signedPercent(summary.paper_return),
            note: SCREEN.strategies.paperReturnNote
        },
        currentState: {
            label: SCREEN.strategies.currentState,
            value: SCREEN.strategies.deviationCount(summary.deviations)
        }
    };
}

/** The validation state, and how many days are left when it is still observing. */
function statusLabel(summary: StrategySummary): string
{
    const state = PAPER_VALIDATION[summary.validation];
    const remaining = summary.observation.total_days - summary.observation.elapsed_days;

    if (summary.validation !== "running" || remaining <= 0)
    {
        return state;
    }

    return `${state} · ${SCREEN.strategies.remaining(remaining)}`;
}

function statusTone(validation: PaperValidationState): TagTone
{
    if (validation === "running")
    {
        return "success";
    }

    return validation === "discarded" ? "warning" : "neutral";
}

/**
 * The card's four states, from the paper lifecycle's six.
 *
 * `draft` and `ready` are both a registration nothing has happened to yet, and
 * `extended` is a run that is still observing past the window it declared. The
 * distinction `extended` carries is not lost by this mapping — the tag beside
 * the name says `관측 기간 연장` in words — but the card has no state of its own
 * for it, which is a gap in the visual contract rather than in this screen.
 */
function cardState(validation: PaperValidationState): StrategyCardState
{
    if (validation === "running" || validation === "extended")
    {
        return "running";
    }

    if (validation === "completed")
    {
        return "completed";
    }

    return validation === "discarded" ? "discarded" : "ready";
}
