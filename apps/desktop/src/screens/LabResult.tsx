import { Button, EmptyState, InlineFeedback, Metric, Panel, Tag, ViewLead } from "@trdr/ui";
import type { TagTone } from "@trdr/ui";

import type {
    BacktestAssumptions,
    BacktestMetrics,
    BacktestWarning,
    CurvePoint,
    Verdict
} from "../bindings";
import { SCREEN, VERDICT, VERDICT_HEADLINE, warningSentence } from "../copy/ko";
import { decimal, percent, shortHash, signedPercent, won } from "../copy/format";
import { labResultRequest, useModel } from "../ipc/model";
import { Screen } from "./Screen";

/**
 * What a backtest run produced (`LabResultModel/v1`).
 *
 * The recipe is TopBar, ViewLead, Metric, a chart Panel, Tag, Button and the
 * approval dialog that belongs to M6. The verdict is a value with three
 * possibilities and the headline for each is in the localisation table; nothing
 * on this screen writes a sentence about a number.
 *
 * No metric here wears a market colour, and that is deliberate rather than an
 * omission. `BacktestMetrics` carries no `MarketTone` — unlike a holding, which
 * does — so there is nothing to read a direction from, and reading one from the
 * sign is exactly what the models exist to prevent. A drawdown is rendered as
 * the magnitude it is stored as, without a sign this screen invented.
 */
export function LabResult()
{
    const state = useModel(labResultRequest());

    return (
        <Screen title={SCREEN.labResult.title} context={SCREEN.labResult.context} state={state}>
            {(model) => (
                <>
                    <ViewLead
                        pill={model.run}
                        eyebrow={SCREEN.labResult.inputHash(shortHash(model.input_hash))}
                        headline={VERDICT_HEADLINE[model.verdict]}
                        status={<Tag tone={verdictTone(model.verdict)} label={VERDICT[model.verdict]} />}
                    />

                    <Warnings warnings={model.warnings} />
                    <ResultMetrics metrics={model.metrics} />
                    <Curve curve={model.curve} />
                    <Assumptions
                        assumptions={model.assumptions}
                        outputHash={shortHash(model.output_hash)}
                    />

                    <InlineFeedback message={SCREEN.labResult.registerNote} />
                    <Button variant="primary" label={SCREEN.labResult.register} disabled />
                </>
            )}
        </Screen>
    );
}

/** Only a sound run is one whose assumptions all held. */
function verdictTone(verdict: Verdict): TagTone
{
    return verdict === "sound" ? "success" : "warning";
}

/**
 * Everything that should temper reading the metrics.
 *
 * A warning arrives as a stable code and a list of safe parameters, never as a
 * sentence — section 11 keeps the wording on this side of the bridge and section
 * 12 keeps the parameters small enough that none of them can be an upstream
 * body. The table turns the pair into Korean.
 */
function Warnings(props: { readonly warnings: readonly BacktestWarning[] })
{
    return (
        <>
            {props.warnings.map((warning) => (
                <InlineFeedback
                    key={`${warning.code}:${warning.params.join(",")}`}
                    tone="warning"
                    message={warningSentence(warning.code, warning.params)}
                />
            ))}
        </>
    );
}

/** The four numbers a result is read through. */
function ResultMetrics(props: { readonly metrics: BacktestMetrics })
{
    const { metrics } = props;

    return (
        <div className="trdr-metric-grid">
            <Panel>
                <Metric
                    label={SCREEN.labResult.totalReturn}
                    value={signedPercent(metrics.total_return)}
                    comparison={SCREEN.labResult.annualised(signedPercent(metrics.annualised_return))}
                />
            </Panel>
            <Panel>
                <Metric label={SCREEN.labResult.maxDrawdown} value={percent(metrics.max_drawdown)} />
            </Panel>
            <Panel>
                <Metric
                    label={SCREEN.labResult.winRate}
                    value={percent(metrics.win_rate)}
                    comparison={SCREEN.labResult.trades(metrics.trades)}
                />
            </Panel>
            <Panel>
                <Metric label={SCREEN.labResult.sharpe} value={decimal(metrics.sharpe)} />
            </Panel>
        </div>
    );
}

/** The equity curve's period and its two ends, until M4 draws the chart. */
function Curve(props: { readonly curve: readonly CurvePoint[] })
{
    const { curve } = props;
    const first = curve[0];
    const last = curve[curve.length - 1];

    if (first === undefined || last === undefined)
    {
        return (
            <Panel title={SCREEN.labResult.curve} caption={SCREEN.labResult.curveCaption}>
                <EmptyState
                    title={SCREEN.labResult.curveEmpty}
                    explanation={SCREEN.labResult.curveEmptyWhy}
                />
            </Panel>
        );
    }

    return (
        <Panel title={SCREEN.labResult.curve} caption={SCREEN.labResult.curveCaption}>
            <div className="trdr-metric-grid">
                <Metric
                    label={SCREEN.labResult.curvePeriod}
                    value={SCREEN.labResult.curveRange(first.date, last.date)}
                    comparison={SCREEN.labResult.curvePoints(curve.length)}
                />
                <Metric
                    label={SCREEN.labResult.curveEquity}
                    value={SCREEN.labResult.curveEnds(won(first.equity), won(last.equity))}
                />
            </div>
        </Panel>
    );
}

/** What the engine assumed, which is half of what makes a number mean anything. */
function Assumptions(props: {
    readonly assumptions: BacktestAssumptions;
    readonly outputHash: string;
})
{
    const { assumptions, outputHash } = props;

    return (
        <Panel title={SCREEN.labResult.assumptions} caption={SCREEN.labResult.assumptionsCaption}>
            <div className="trdr-metric-grid">
                <Metric label={SCREEN.labResult.fill} value={assumptions.fill} />
                <Metric label={SCREEN.labResult.commission} value={assumptions.commission} />
                <Metric label={SCREEN.labResult.slippage} value={assumptions.slippage} />
                <Metric label={SCREEN.labResult.tax} value={assumptions.tax} />
                <Metric label={SCREEN.labResult.engine} value={assumptions.engine_version} />
                <Metric label={SCREEN.labResult.outputHash} value={outputHash} />
            </div>
        </Panel>
    );
}
