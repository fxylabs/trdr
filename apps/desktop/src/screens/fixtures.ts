/**
 * The models the screen tests render, shaped like the ones the host sends.
 *
 * These mirror `fixtures/synthetic/domain-v1/domain.json` as the query service
 * turns it into models: the same instruments, the same states, the same
 * `SYN`-prefixed symbols that cannot be mistaken for a listed security. What
 * they are not is a second copy of that file — a test that needed the fixture's
 * exact numbers would be testing the fixture, and the assertions here are about
 * what a screen does with a shape.
 *
 * `answers` and `refuses` stand in for a command. `answers` echoes the request
 * id back the way a correct host does, which is what lets a test assert that a
 * screen checks it: the one test that cares supplies its own wrong id instead.
 */

import type {
    ErrorCode,
    LabDraftModel,
    LabResultModel,
    StrategiesModel,
    StrategyDetailModel,
    StrategyRules,
    TodayModel,
    UiResponseEnvelope_Serialize
} from "../bindings";

const BUILT_AT = "2026-08-11T00:30:00Z";

const RULES: StrategyRules =
{
    universe: [{ label: "시장", value: "SYN" }],
    entry: [{ label: "신호", value: "종가 > 20일 이동평균" }],
    exit_and_cost: [{ label: "청산", value: "종가 < 10일 이동평균" }],
    validation_window: [{ label: "관측 기간", value: "60 거래일" }],
    section: "draft"
};

export const TODAY: TodayModel =
{
    model: "TodayModel/v1",
    origin: "synthetic",
    built_at: BUILT_AT,
    account:
    {
        total_value: 12_480_000,
        cash: 2_180_000,
        day_change: 142_000,
        day_change_ratio: "0.0115",
        day_tone: "up",
        as_of: "2026-08-10T15:30:00Z",
        state: "ready",
        connection: "connected-read-only"
    },
    holdings: [
        {
            symbol: "SYN0001",
            name: "합성전자",
            quantity: 320,
            average_price: 9_800,
            last_price: 10_150,
            market_value: 3_248_000,
            unrealized: 112_000,
            unrealized_ratio: "0.0357",
            tone: "up"
        },
        {
            /* Gained in won and marked `down` by the model. A screen that read
               the direction off the sign would paint this one red. */
            symbol: "SYN0002",
            name: "합성화학",
            quantity: 40,
            average_price: 21_050,
            last_price: 20_950,
            market_value: 838_000,
            unrealized: 4_000,
            unrealized_ratio: "0.0048",
            tone: "down"
        }
    ],
    holdings_state: "ready",
    events: [
        {
            kind: "disclosure",
            title: "주요사항보고서 (합성 공시)",
            summary: "disclosure-for-holding",
            at: "2026-08-10T07:40:00Z",
            subject: "SYN0001"
        },
        {
            kind: "validation-progress",
            title: "2026-08-10",
            summary: "observation-day-recorded",
            at: "2026-08-10T15:35:00Z",
            subject: "syn-meanrev"
        }
    ],
    events_state: "ready"
};

export const LAB_DRAFT: LabDraftModel =
{
    model: "LabDraftModel/v1",
    origin: "synthetic",
    built_at: BUILT_AT,
    strategy: "syn-momentum",
    name: "합성 모멘텀",
    source_path: "strategies/syn-momentum.trdr.yaml",
    rules: RULES,
    period_start: "2026-05-01",
    period_end: "2026-08-07",
    support: "missing-data",
    coverage: [
        {
            source: "user.synthetic",
            total_days: 68,
            covered_days: 63,
            gaps: [
                { from: "2026-06-15", to: "2026-06-17" },
                { from: "2026-07-28", to: "2026-07-29" }
            ]
        }
    ],
    coverage_state: "stale"
};

export const LAB_RESULT: LabResultModel =
{
    model: "LabResultModel/v1",
    origin: "synthetic",
    built_at: BUILT_AT,
    run: "syn-run-0001",
    strategy: "syn-momentum",
    rules: { ...RULES, section: "frozen" },
    metrics:
    {
        total_return: "0.0412",
        annualised_return: "0.1030",
        max_drawdown: "0.0870",
        sharpe: "0.6200",
        trades: 14,
        win_rate: "0.5710"
    },
    curve: [
        { date: "2026-05-04", equity: 10_000_000 },
        { date: "2026-08-07", equity: 10_412_000 }
    ],
    assumptions:
    {
        fill: "next-open",
        commission: "0.015%",
        slippage: "1 tick",
        tax: "0.20% on sale",
        engine_version: "synthetic-0"
    },
    input_hash: "3f1c9a04d6b25e8710c4af93b2d5e6087a1c4d9f2e3b508716ca9d4f2b3e6c81",
    output_hash: "9b04e7c2d13a56f8402b9e7c1d5a3f6089c2b4e70d13a58f26b9c4e70d1a35f8",
    warnings: [{ code: "DATA_INCOMPLETE", params: ["2026-06-15", "2026-06-17"] }],
    verdict: "qualified"
};

export const STRATEGIES: StrategiesModel =
{
    model: "StrategiesModel/v1",
    origin: "synthetic",
    built_at: BUILT_AT,
    strategies: [
        {
            strategy: "syn-meanrev",
            name: "합성 평균회귀",
            description: "5일 하락 후 반등을 관측한다",
            validation: "running",
            observation: { elapsed_days: 9, total_days: 60, today_index: 9 },
            paper_return: "0.0180",
            deviations: 1
        },
        {
            strategy: "syn-breakout",
            name: "합성 돌파",
            description: "60일 신고가 돌파를 관측한다",
            validation: "completed",
            observation: { elapsed_days: 60, total_days: 60, today_index: null },
            paper_return: "-0.0240",
            deviations: 0
        }
    ],
    state: "ready"
};

export const STRATEGY_DETAIL: StrategyDetailModel =
{
    model: "StrategyDetailModel/v1",
    origin: "synthetic",
    built_at: BUILT_AT,
    strategy: "syn-meanrev",
    name: "합성 평균회귀",
    rules: { ...RULES, section: "frozen" },
    validation: "running",
    observation: { elapsed_days: 9, total_days: 60, today_index: 9 },
    positions: [
        {
            symbol: "SYN0002",
            name: "합성화학",
            quantity: 40,
            entry_price: 21_050,
            last_price: 20_950,
            entered_on: "2026-08-05"
        }
    ],
    positions_state: "ready",
    signals: [
        {
            action: "entry",
            symbol: "SYN0002",
            rule: "5일 연속 하락",
            at: "2026-08-05T00:10:00Z",
            market_date: "2026-08-05"
        }
    ],
    signals_state: "ready",
    lineage:
    {
        registered_at: "2026-07-29T02:14:00Z",
        supersedes: null,
        approved_run: "syn-run-0002",
        spec_hash: "c4e70d1a35f89b04e7c2d13a56f8402b9e7c1d5a3f6089c2b4e70d13a58f26b9"
    },
    deviations: [{ market_date: "2026-08-06", rule: "종목당 8%", code: "DATA_INCOMPLETE" }]
};

/** A host that answers, and answers the request it was actually asked. */
export function answers<Model>(model: Model)
{
    return (id: string): Promise<UiResponseEnvelope_Serialize<Model>> =>
        Promise.resolve({ v: 1, id, outcome: { status: "ok", value: model } });
}

/** A host that refuses, in the envelope a refusal comes in. */
export function refuses<Model>(code: ErrorCode)
{
    return (id: string): Promise<UiResponseEnvelope_Serialize<Model>> =>
        Promise.resolve({
            v: 1,
            id,
            outcome: { status: "error", value: { v: 1, code, retryability: { kind: "no" } } }
        });
}
