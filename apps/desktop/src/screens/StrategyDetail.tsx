import { useNavigate, useParams } from "react-router";
import {
    DataTable,
    DayProgress,
    EmptyState,
    Metric,
    Panel,
    RuleGrid,
    SignalLog,
    StockCell,
    Tag
} from "@trdr/ui";
import type { DataTableColumn, SignalContent } from "@trdr/ui";

import type {
    ObservationProgress,
    PaperPosition,
    PaperValidationState,
    RuleDeviation,
    Signal,
    StrategyDetailModel,
    StrategyLineage
} from "../bindings";
import { NAV, PAPER_VALIDATION, SCREEN, shortWords, words } from "../copy/ko";
import { clockTime, count, instant, share, shares, shortHash, won } from "../copy/format";
import { strategyRequest, useModel } from "../ipc/model";
import type { ModelState } from "../ipc/model";
import { paths } from "../shell/routes";
import { Screen } from "./Screen";
import { SectionNote, metricState, signalState, tableState } from "./sections";
import { ruleSections } from "./rules";

/**
 * One registered strategy in full (`StrategyDetailModel/v1`).
 *
 * The recipe is TopBar with a back control, Metric, a progress Panel, a frozen
 * RuleGrid, DataTable, DayProgress and SignalLog. The back control is why this
 * view has a route of its own with the strategy's id in it: a detail view
 * always exposes where it came from, and a screen that was pushed into place
 * without a URL has nowhere to go back to and nothing for `trdr ui open` to
 * point at later.
 *
 * The rules are frozen, and `RuleGrid` is told so rather than left to infer it.
 * A grid that offered an edit while a paper validation was running would make
 * the validation's result meaningless, so the affordance is absent rather than
 * refused.
 */
export function StrategyDetail()
{
    const navigate = useNavigate();
    const { strategy = "" } = useParams();
    const state = useModel(strategyRequest(strategy));

    return (
        <Screen
            title={heading(state)}
            context={strategy}
            back={{ label: SCREEN.strategyDetail.back, onActivate: () => navigate(paths.strategies) }}
            state={state}
        >
            {(model) => (
                <>
                    <DetailMetrics model={model} />

                    <Panel
                        title={SCREEN.strategyDetail.progress}
                        caption={SCREEN.strategyDetail.progressCaption(model.observation.total_days)}
                        end={<Tag label={PAPER_VALIDATION[model.validation]} />}
                    >
                        <Progress observation={model.observation} validation={model.validation} />
                    </Panel>

                    <Panel
                        title={SCREEN.strategyDetail.rules}
                        caption={SCREEN.strategyDetail.rulesCaption}
                        end={<Tag tone="success" label={SCREEN.strategyDetail.frozen} />}
                    >
                        <RuleGrid {...ruleSections(model.rules)} state="frozen" />
                    </Panel>

                    <SectionNote state={model.positions_state} />
                    <Panel
                        title={SCREEN.strategyDetail.positions}
                        caption={SCREEN.strategyDetail.positionsCaption}
                    >
                        <DataTable
                            label={SCREEN.strategyDetail.positions}
                            columns={POSITION_COLUMNS}
                            rows={model.positions}
                            rowId={(row) => row.symbol}
                            state={tableState(model.positions_state)}
                            empty={
                                <EmptyState
                                    title={SCREEN.strategyDetail.emptyPositions}
                                    explanation={SCREEN.strategyDetail.emptyPositionsWhy}
                                />
                            }
                        />
                    </Panel>

                    <SectionNote state={model.signals_state} />
                    <Panel
                        title={SCREEN.strategyDetail.signalLog}
                        caption={SCREEN.strategyDetail.signalLogCaption}
                    >
                        <SignalLog
                            label={SCREEN.strategyDetail.signalLog}
                            signals={model.signals}
                            signalId={signalId}
                            signal={signalContent}
                            state={signalState(model.signals_state)}
                            empty={
                                <EmptyState
                                    title={SCREEN.strategyDetail.emptySignals}
                                    explanation={SCREEN.strategyDetail.emptySignalsWhy}
                                />
                            }
                        />
                    </Panel>

                    <Deviations deviations={model.deviations} />
                    <Lineage lineage={model.lineage} />
                </>
            )}
        </Screen>
    );
}

/** The strategy's name once it is known, and what the view is until then. */
function heading(state: ModelState<StrategyDetailModel>): string
{
    return state.status === "ready" ? state.model.name : NAV.strategyDetail;
}

/** The four numbers a registration is read through. None of them is a return. */
function DetailMetrics(props: { readonly model: StrategyDetailModel })
{
    const { model } = props;
    const { elapsed_days: elapsed, total_days: total } = model.observation;

    return (
        <div className="trdr-metric-grid">
            <Panel>
                <Metric
                    label={SCREEN.strategyDetail.observation}
                    value={SCREEN.strategies.observationValue(elapsed, total)}
                    comparison={SCREEN.strategies.observationNote(share(elapsed, total))}
                />
            </Panel>
            <Panel>
                <Metric
                    label={SCREEN.strategyDetail.deviations}
                    value={count(model.deviations.length)}
                />
            </Panel>
            <Panel>
                <Metric
                    label={SCREEN.strategyDetail.signals}
                    value={count(model.signals.length)}
                    state={metricState(model.signals_state)}
                />
            </Panel>
            <Panel>
                <Metric
                    label={SCREEN.strategyDetail.positions}
                    value={count(model.positions.length)}
                    state={metricState(model.positions_state)}
                />
            </Panel>
        </div>
    );
}

/**
 * How far through the declared window the run is.
 *
 * `today_index` is absent outside market days, and the bar is told so rather
 * than being given a segment that is only nearly today. `DayProgress` has two
 * states where the lifecycle has six; the tag beside the panel's title carries
 * the distinction the bar cannot.
 */
function Progress(props: {
    readonly observation: ObservationProgress;
    readonly validation: PaperValidationState;
})
{
    const { observation, validation } = props;

    return (
        <DayProgress
            elapsed={observation.elapsed_days}
            total={observation.total_days}
            {...(observation.today_index === null ? {} : { today: observation.today_index })}
            state={validation === "completed" ? "completed" : "running"}
            label={SCREEN.strategyDetail.progressLabel(
                observation.elapsed_days,
                observation.total_days
            )}
        />
    );
}

/** Paper positions. No market tone: the model carries none, so none is invented. */
const POSITION_COLUMNS: readonly DataTableColumn<PaperPosition>[] = [
    {
        id: "symbol",
        header: SCREEN.strategyDetail.columnSymbol,
        width: "40%",
        cell: (row) => (
            <StockCell
                symbol={row.symbol.slice(0, 3)}
                name={row.name}
                summary={SCREEN.strategyDetail.positionSummary(row.symbol, row.entered_on)}
            />
        )
    },
    {
        id: "quantity",
        header: SCREEN.strategyDetail.columnQuantity,
        cell: (row) => shares(row.quantity)
    },
    {
        id: "entryPrice",
        header: SCREEN.strategyDetail.columnEntryPrice,
        cell: (row) => won(row.entry_price)
    },
    {
        id: "lastPrice",
        header: SCREEN.strategyDetail.columnLastPrice,
        cell: (row) => won(row.last_price)
    }
];

/** A signal carries no id, so one is made from what identifies it. */
function signalId(signal: Signal): string
{
    return `${signal.at}:${signal.symbol}:${signal.action}`;
}

/** Every signal names the registered rule it came from. That is not optional. */
function signalContent(signal: Signal): SignalContent
{
    return {
        title: SCREEN.strategyDetail.signalTitle(words(signal.action), signal.symbol),
        explanation: `${signal.market_date} · ${SCREEN.strategyDetail.signalRule(signal.rule)}`,
        timestamp: signal.at,
        timeLabel: clockTime(signal.at)
    };
}

const DEVIATION_COLUMNS: readonly DataTableColumn<RuleDeviation>[] = [
    { id: "date", header: SCREEN.strategyDetail.columnDate, cell: (row) => row.market_date },
    { id: "rule", header: SCREEN.strategyDetail.columnRule, cell: (row) => row.rule },
    { id: "reason", header: SCREEN.strategyDetail.columnReason, cell: (row) => shortWords(row.code) }
];

/** What the run did that the registered rules did not say. */
function Deviations(props: { readonly deviations: readonly RuleDeviation[] })
{
    const { deviations } = props;

    return (
        <Panel
            title={SCREEN.strategyDetail.deviationLog}
            caption={SCREEN.strategyDetail.deviationCaption}
        >
            <DataTable
                label={SCREEN.strategyDetail.deviationLog}
                columns={DEVIATION_COLUMNS}
                rows={deviations}
                rowId={(row) => `${row.market_date}:${row.rule}:${row.code}`}
                state={deviations.length === 0 ? "empty" : "ready"}
                empty={
                    <EmptyState
                        title={SCREEN.strategyDetail.emptyDeviations}
                        explanation={SCREEN.strategyDetail.emptyDeviationsWhy}
                    />
                }
            />
        </Panel>
    );
}

/** Where this registration came from, and what it replaced. */
function Lineage(props: { readonly lineage: StrategyLineage })
{
    const { lineage } = props;

    return (
        <Panel
            title={SCREEN.strategyDetail.lineage}
            caption={SCREEN.strategyDetail.lineageCaption}
        >
            <div className="trdr-metric-grid">
                <Metric
                    label={SCREEN.strategyDetail.registeredAt}
                    value={instant(lineage.registered_at)}
                />
                <Metric
                    label={SCREEN.strategyDetail.supersedes}
                    value={lineage.supersedes ?? SCREEN.strategyDetail.none}
                />
                <Metric
                    label={SCREEN.strategyDetail.approvedRun}
                    value={lineage.approved_run ?? SCREEN.strategyDetail.none}
                />
                <Metric
                    label={SCREEN.strategyDetail.specHash}
                    value={shortHash(lineage.spec_hash)}
                />
            </div>
        </Panel>
    );
}
