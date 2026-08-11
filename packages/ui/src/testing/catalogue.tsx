/**
 * Every shipped role, in every state its contract gives it.
 *
 * One list, read by two things that would otherwise drift apart: the tests,
 * which assert that each rendering carries the contract's class, its exact state
 * string and the structure the stylesheet selects on, and the gallery, which
 * shows a person the same renderings. A state added to `contracts.v2.json` and
 * not added here fails `contracts.test.tsx`; a state added here appears on the
 * gallery page without anyone remembering to put it there.
 *
 * The copy is Korean because the product is, and because a component that only
 * ever holds `Lorem ipsum` has not been looked at in the language it will carry.
 */

import type { ReactElement } from "react";

import type { DataTableColumn, EventContent, MarketValueTone, SignalContent } from "../index";
import {
    AppShell,
    BrokerConnection,
    Button,
    CoverageCard,
    DataTable,
    DayProgress,
    EmptyState,
    EventList,
    Field,
    InlineFeedback,
    Metric,
    NavigationItem,
    Panel,
    RuleGrid,
    Sidebar,
    SignalLog,
    StockCell,
    StrategyCard,
    Tag,
    TopBar,
    ViewLead
} from "../index";

export type RoleCase =
{
    /** The contract state this rendering is in, or the default when absent. */
    state?: string;
    element: ReactElement;
    /** True when the state is "the component is gone". */
    absent?: boolean;
    /** Kit selectors this state in particular has to match. */
    selectors?: readonly string[];
};

export type Role =
{
    /** The key in `contracts.v2.json`. */
    contract: string;
    /** Gallery only: the role needs the full width of the page to be judged. */
    wide?: boolean;
    /** States expressed by a CSS pseudo-class, which no prop can put the DOM in. */
    pseudoStates?: readonly string[];
    /** Kit selectors the first case has to match. */
    selectors: readonly string[];
    cases: readonly RoleCase[];
};

type Holding =
{
    id: string;
    symbol: string;
    name: string;
    position: string;
    value: string;
    profit: string;
    profitRate: string;
    today: string;
    tone: MarketValueTone;
    todayTone: MarketValueTone;
};

const HOLDINGS: readonly Holding[] = [
    {
        id: "005930",
        symbol: "삼성",
        name: "삼성전자",
        position: "40주 · 평균 71,250원",
        value: "3,492,000원",
        profit: "+642,000원",
        profitRate: "+22.53%",
        today: "+1.12%",
        tone: "up",
        todayTone: "up"
    },
    {
        id: "005380",
        symbol: "현대",
        name: "현대차",
        position: "12주 · 평균 216,500원",
        value: "2,534,400원",
        profit: "−63,600원",
        profitRate: "−2.45%",
        today: "+0.38%",
        tone: "down",
        todayTone: "up"
    },
    {
        id: "035420",
        symbol: "NAV",
        name: "NAVER",
        position: "18주 · 평균 184,100원",
        value: "3,521,700원",
        profit: "+207,900원",
        profitRate: "+6.27%",
        today: "−0.74%",
        tone: "up",
        todayTone: "down"
    }
];

/** A market value with its tone on the value and on nothing else. */
function Money(props: { tone: MarketValueTone; value: string; sub?: string })
{
    return (
        <span className={props.tone === "up" ? "trdr-market-up" : "trdr-market-down"}>
            {props.value}
            {props.sub === undefined ? null : <span className="trdr-data-sub">{props.sub}</span>}
        </span>
    );
}

export const HOLDING_COLUMNS: readonly DataTableColumn<Holding>[] = [
    {
        id: "instrument",
        header: "종목",
        width: "40%",
        cell: (row) => <StockCell symbol={row.symbol} name={row.name} summary={row.position} />
    },
    { id: "value", header: "평가금액", sortable: true, cell: (row) => row.value },
    {
        id: "profit",
        header: "총손익",
        sortable: true,
        cell: (row) => <Money tone={row.tone} value={row.profit} sub={row.profitRate} />
    },
    { id: "today", header: "오늘", cell: (row) => <Money tone={row.todayTone} value={row.today} /> }
];

type Disclosure = { id: string; kind: string; headline: string; detail: string };

const EVENTS: readonly Disclosure[] = [
    { id: "d1", kind: "공시", headline: "삼성전자 신규 공시", detail: "주요사항보고서가 14:28 도착했습니다." },
    { id: "d2", kind: "전략", headline: "저변동 모멘텀 · 조건 이탈", detail: "NAVER가 보유 조건에서 벗어났습니다." },
    { id: "d3", kind: "D−12", headline: "모의검증 진행 중", detail: "20일 중 8일 관측했습니다." }
];

const eventContent = (item: Disclosure): EventContent => ({
    icon: item.kind,
    title: item.headline,
    summary: item.detail
});

type SignalRecord = { id: string; title: string; at: string; clock: string; why: string };

const SIGNALS: readonly SignalRecord[] = [
    {
        id: "s1",
        title: "NAVER 청산 신호",
        at: "2026-08-11T14:31:00+09:00",
        clock: "14:31",
        why: "등록 규칙 MA20 < MA60에 해당해 보유 조건에서 벗어났습니다."
    },
    {
        id: "s2",
        title: "삼성전자 편입 유지",
        at: "2026-08-11T09:05:00+09:00",
        clock: "09:05",
        why: "등록 규칙 MA20 > MA60을 계속 만족합니다."
    }
];

const signalContent = (record: SignalRecord): SignalContent => ({
    title: record.title,
    explanation: record.why,
    timestamp: record.at,
    timeLabel: record.clock
});

const RULES =
{
    universe: { heading: "대상과 순위", rules: ["KOSPI 100", "변동성 하위 30%"] },
    entry: { heading: "진입과 비중", rules: ["MA20 > MA60", "최대 10종목"] },
    exitAndCost: { heading: "청산과 비용", rules: ["MA20 < MA60", "슬리피지 0.10%"] },
    validationWindow: { heading: "검증 분리", rules: ["학습 70%", "검증 30%"] }
};

const STRATEGY =
{
    id: "low-vol-v1",
    name: "대형주 저변동 모멘텀",
    description: "KOSPI 100 · 일간 평가",
    status: <Tag tone="success" label="모의검증 중 · D−12" />,
    observation: { label: "관측", value: "8 / 20일", note: "40%" },
    paperReturn: { label: "모의 수익률", value: "+2.1%", note: "등록 이후" },
    currentState: { label: "상태", value: "이탈 1", note: "NAVER" }
};

const EMPTY = (
    <EmptyState
        title="아직 등록된 전략이 없습니다."
        explanation="실험실에서 백테스트 결과를 확인한 뒤 모의검증에 등록할 수 있습니다."
        action={<Button variant="primary" label="실험실 열기" />}
    />
);

export const CATALOGUE: readonly Role[] = [
    {
        contract: "AppShell",
        wide: true,
        selectors: [".trdr-app-shell", ".trdr-workspace", ".trdr-agent-rail"],
        cases: [
            {
                state: "ready",
                element: (
                    <AppShell
                        activeArea="today"
                        sidebar={<Sidebar brand="trdr" items={[{ id: "today", label: "Today", current: true }]} />}
                        agentRail={<div />}
                    >
                        <TopBar title="Today" />
                    </AppShell>
                )
            }
        ]
    },
    {
        contract: "Sidebar",
        selectors: [".trdr-sidebar", ".trdr-brand", ".trdr-nav", ".trdr-nav-item"],
        cases: [
            {
                state: "ready",
                element: (
                    <Sidebar
                        brand="trdr"
                        items={[
                            { id: "today", label: "Today", current: true },
                            { id: "lab", label: "Lab" },
                            { id: "strategies", label: "Strategies" }
                        ]}
                        connection={
                            <BrokerConnection
                                broker="KIS"
                                state="connected-read-only"
                                stateLabel="연결됨"
                                authority="읽기 전용"
                                lastSync="14:31 동기화"
                            />
                        }
                    />
                )
            }
        ]
    },
    {
        contract: "NavigationItem",
        pseudoStates: ["hover", "focus-visible"],
        selectors: [".trdr-nav-item"],
        cases: [
            { state: "rest", element: <NavigationItem label="Lab" /> },
            {
                state: "current",
                element: <NavigationItem label="Today" current />,
                selectors: [
                    '.trdr-nav-item[aria-current="page"]',
                    '.trdr-nav-item[aria-current="page"]::before',
                    '.trdr-nav-item[aria-current="page"]::after'
                ]
            }
        ]
    },
    {
        contract: "BrokerConnection",
        selectors: [".trdr-connection", ".trdr-connection strong", ".trdr-connection strong::before"],
        cases: [
            {
                state: "connected-read-only",
                element: (
                    <BrokerConnection
                        broker="KIS"
                        state="connected-read-only"
                        stateLabel="연결됨"
                        authority="읽기 전용"
                        lastSync="14:31 동기화"
                    />
                )
            },
            {
                state: "syncing",
                element: <BrokerConnection broker="KIS" state="syncing" stateLabel="동기화 중" authority="읽기 전용" />,
                selectors: ['.trdr-connection[data-state="syncing"] strong::before']
            },
            {
                state: "stale",
                element: (
                    <BrokerConnection
                        broker="KIS"
                        state="stale"
                        stateLabel="지연됨 · 12분 전 값"
                        authority="읽기 전용"
                    />
                ),
                selectors: ['.trdr-connection[data-state="stale"] strong::before']
            },
            {
                state: "disconnected",
                element: (
                    <BrokerConnection
                        broker="KIS"
                        state="disconnected"
                        stateLabel="연결 끊김"
                        onOpenSettings={() => undefined}
                        settingsLabel="설정 열기"
                    />
                ),
                selectors: ['.trdr-connection[data-state="disconnected"] strong::before']
            },
            {
                state: "error",
                element: (
                    <BrokerConnection
                        broker="KIS"
                        state="error"
                        stateLabel="인증 실패"
                        onRetry={() => undefined}
                        retryLabel="다시 시도"
                    />
                ),
                selectors: ['.trdr-connection[data-state="error"] strong::before']
            }
        ]
    },
    {
        contract: "TopBar",
        wide: true,
        selectors: [".trdr-topbar", ".trdr-topbar h1", ".trdr-topbar-context", ".trdr-topbar-meta", ".trdr-avatar"],
        cases: [
            {
                state: "ready",
                element: (
                    <TopBar
                        title="전략 상세"
                        context="low-vol-v1"
                        metadata="마지막 평가 14:31 · 로컬 보관"
                        avatar="RY"
                        back={{ label: "전략 목록", onActivate: () => undefined }}
                    />
                ),
                selectors: [".trdr-back-button"]
            }
        ]
    },
    {
        contract: "ViewLead",
        wide: true,
        selectors: [".trdr-view-lead", ".trdr-view-lead-row", ".trdr-lead-pill", ".trdr-view-lead h2"],
        cases: [
            {
                state: "ready",
                element: (
                    <ViewLead
                        pill="Agent가 생성한 초안"
                        eyebrow="실행 전에 확인합니다"
                        headline="규칙을 재현 가능한 형식으로 고정합니다."
                        description="정보 위계는 크기, 여백, border로 만들고 lime은 현재 지점만 가리킵니다."
                        status={<Tag tone="focus" label="초안 v1" />}
                    />
                )
            }
        ]
    },
    {
        contract: "Panel",
        selectors: [".trdr-panel", ".trdr-panel-header", ".trdr-panel-header h3", ".trdr-panel-header-end"],
        cases: [
            {
                state: "rest",
                element: (
                    <Panel title="실행 규칙" caption="strategy.yaml" end={<Tag label="초안 v1" />}>
                        <RuleGrid {...RULES} />
                    </Panel>
                )
            },
            {
                state: "loading",
                element: (
                    <Panel title="백테스트" caption="실행 중" state="loading">
                        <i className="trdr-skeleton trdr-skeleton-line" />
                    </Panel>
                )
            },
            {
                state: "empty",
                element: (
                    <Panel state="empty">
                        {EMPTY}
                    </Panel>
                )
            },
            {
                state: "error",
                element: (
                    <Panel title="데이터 커버리지" state="error">
                        <InlineFeedback tone="error" message="커버리지를 계산하지 못했습니다." />
                    </Panel>
                )
            }
        ]
    },
    {
        contract: "Button",
        pseudoStates: ["hover", "focus-visible", "pressed"],
        selectors: [".trdr-button"],
        cases: [
            { state: "rest", element: <Button variant="primary" label="검증 시작" />, selectors: [".trdr-button--primary", ".trdr-button--primary::before"] },
            { state: "disabled", element: <Button label="비활성" disabled />, selectors: [".trdr-button:disabled"] }
        ]
    },
    {
        contract: "Tag",
        selectors: [".trdr-tag"],
        cases: [{ state: "rest", element: <Tag tone="success" label="연결됨" />, selectors: [".trdr-tag--success"] }]
    },
    {
        contract: "Field",
        pseudoStates: ["focus-visible"],
        selectors: [".trdr-field", ".trdr-field-group", ".trdr-field-group > .trdr-field", ".trdr-field-label"],
        cases: [
            {
                state: "rest",
                element: <Field label="전략 이름" value="대형주 저변동 모멘텀" hint="등록 후에는 고정됩니다." />
            },
            {
                state: "invalid",
                element: <Field label="관측 일수" value="0" error="1일 이상이어야 합니다." />,
                selectors: ['.trdr-field[aria-invalid="true"]', ".trdr-field-error"]
            },
            {
                state: "disabled",
                element: <Field label="전략 이름" value="대형주 저변동 모멘텀" disabled />
            }
        ]
    },
    {
        contract: "Metric",
        selectors: [".trdr-metric", ".trdr-metric label", ".trdr-metric strong", ".trdr-metric small"],
        cases: [
            {
                state: "ready",
                element: <Metric label="검증 수익률" value="+8.4%" comparison="학습 +15.7%" marketTone="up" />,
                selectors: [".trdr-num", ".trdr-market-up"]
            },
            { state: "loading", element: <Metric label="최대 낙폭" value="—" state="loading" /> },
            { state: "unavailable", element: <Metric label="초과수익" value="—" comparison="기준 지수 없음" state="unavailable" /> }
        ]
    },
    {
        contract: "DataTable",
        wide: true,
        selectors: [
            ".trdr-data-table",
            ".trdr-data-table th",
            ".trdr-data-table td",
            ".trdr-data-table th:first-child",
            ".trdr-data-table td:first-child",
            ".trdr-data-table tr:last-child td"
        ],
        cases: [
            {
                state: "ready",
                element: (
                    <DataTable
                        label="보유 종목"
                        columns={HOLDING_COLUMNS}
                        rows={HOLDINGS}
                        rowId={(row) => row.id}
                        sort={{ columnId: "profit", direction: "descending" }}
                        onSort={() => undefined}
                    />
                ),
                selectors: [".trdr-stock-cell", ".trdr-symbol", ".trdr-data-sub"]
            },
            {
                state: "loading",
                element: (
                    <DataTable
                        label="보유 종목"
                        columns={HOLDING_COLUMNS}
                        rows={[]}
                        rowId={(row) => row.id}
                        state="loading"
                    />
                ),
                selectors: [".trdr-skeleton"]
            },
            {
                state: "empty",
                element: (
                    <DataTable
                        label="보유 종목"
                        columns={HOLDING_COLUMNS}
                        rows={[]}
                        rowId={(row) => row.id}
                        state="empty"
                        empty={EMPTY}
                    />
                ),
                selectors: [".trdr-empty-state"]
            },
            {
                state: "stale",
                element: (
                    <DataTable
                        label="보유 종목"
                        columns={HOLDING_COLUMNS}
                        rows={HOLDINGS}
                        rowId={(row) => row.id}
                        state="stale"
                    />
                )
            },
            {
                state: "error",
                element: (
                    <DataTable
                        label="보유 종목"
                        columns={HOLDING_COLUMNS}
                        rows={[]}
                        rowId={(row) => row.id}
                        state="error"
                    />
                )
            }
        ]
    },
    {
        contract: "StockCell",
        selectors: [".trdr-stock-cell", ".trdr-symbol", ".trdr-stock-cell b", ".trdr-stock-cell small"],
        cases: [
            {
                state: "ready",
                element: <StockCell symbol="삼성" name="삼성전자" summary="40주 · 평균 71,250원" />
            }
        ]
    },
    {
        contract: "EventList",
        selectors: [".trdr-event-list", ".trdr-event", ".trdr-event-icon", ".trdr-event b", ".trdr-event p"],
        cases: [
            {
                state: "ready",
                element: (
                    <EventList
                        label="오늘의 이벤트"
                        items={EVENTS}
                        itemId={(item) => item.id}
                        event={eventContent}
                        onActivateEvent={() => undefined}
                    />
                )
            },
            {
                state: "loading",
                element: <EventList label="오늘의 이벤트" items={[]} itemId={(item) => item.id} event={eventContent} state="loading" />
            },
            {
                state: "empty",
                element: (
                    <EventList
                        label="오늘의 이벤트"
                        items={[]}
                        itemId={(item) => item.id}
                        event={eventContent}
                        state="empty"
                        empty={
                            <EmptyState
                                title="오늘 도착한 이벤트가 없습니다."
                                explanation="보유 종목의 공시와 전략 신호가 도착하면 여기에 쌓입니다."
                            />
                        }
                    />
                )
            },
            {
                state: "error",
                element: <EventList label="오늘의 이벤트" items={[]} itemId={(item) => item.id} event={eventContent} state="error" />
            }
        ]
    },
    {
        contract: "RuleGrid",
        wide: true,
        selectors: [
            ".trdr-rule-grid",
            ".trdr-rule-section",
            ".trdr-rule-section h4",
            ".trdr-rule-value",
            ".trdr-rule-section:nth-child(even)"
        ],
        cases: [
            {
                state: "draft",
                element: <RuleGrid {...RULES} state="draft" onEdit={() => undefined} editLabel="수정" />
            },
            { state: "frozen", element: <RuleGrid {...RULES} state="frozen" onEdit={() => undefined} editLabel="수정" /> }
        ]
    },
    {
        contract: "CoverageCard",
        selectors: [".trdr-panel", ".trdr-metric", ".trdr-progress-track", ".trdr-progress-track > i"],
        cases: [
            {
                state: "ready",
                element: (
                    <CoverageCard
                        source="KIS 일봉"
                        days={1258}
                        covered={1258}
                        total={1258}
                        summary="1,258 / 1,258일 · 100% · 결측 없음"
                    />
                )
            },
            {
                state: "loading",
                element: <CoverageCard source="KIS 일봉" days={1258} covered={0} total={1258} summary="확인 중" state="loading" />
            },
            {
                state: "incomplete",
                element: (
                    <CoverageCard
                        source="OpenDART 공시"
                        days={1258}
                        covered={1190}
                        total={1258}
                        summary="1,190 / 1,258일 · 94.6% · 68일 결측"
                        state="incomplete"
                        onInspectMissing={() => undefined}
                        inspectLabel="결측 구간 보기"
                    />
                )
            },
            {
                state: "error",
                element: (
                    <CoverageCard
                        source="ECOS 기준금리"
                        days={1258}
                        covered={0}
                        total={1258}
                        summary="커버리지를 계산하지 못했습니다."
                        state="error"
                    />
                )
            }
        ]
    },
    {
        contract: "StrategyCard",
        wide: true,
        selectors: [
            ".trdr-strategy-card",
            ".trdr-strategy-name h3",
            ".trdr-strategy-name p",
            ".trdr-strategy-stat",
            ".trdr-strategy-stat label",
            ".trdr-strategy-stat strong"
        ],
        cases: [
            { state: "ready", element: <StrategyCard strategy={STRATEGY} href="#/strategies/low-vol-v1" /> },
            { state: "running", element: <StrategyCard strategy={STRATEGY} state="running" onOpen={() => undefined} /> },
            {
                state: "completed",
                element: (
                    <StrategyCard
                        strategy={{
                            ...STRATEGY,
                            status: <Tag label="검증 완료" />,
                            observation: { label: "관측", value: "20 / 20일", note: "100%" },
                            currentState: { label: "상태", value: "이탈 0" }
                        }}
                        state="completed"
                        href="#/strategies/low-vol-v1"
                    />
                )
            },
            {
                state: "discarded",
                element: (
                    <StrategyCard
                        strategy={{ ...STRATEGY, status: <Tag tone="warning" label="폐기됨" /> }}
                        state="discarded"
                        href="#/strategies/low-vol-v1"
                    />
                )
            }
        ]
    },
    {
        contract: "DayProgress",
        wide: true,
        selectors: [".trdr-daybar", ".trdr-daybar i", '.trdr-daybar i[data-state="done"]', '.trdr-daybar i[data-state="today"]'],
        cases: [
            { state: "running", element: <DayProgress elapsed={8} total={20} today={8} label="20일 중 8일 관측" /> },
            { state: "completed", element: <DayProgress elapsed={20} total={20} state="completed" label="20일 중 20일 관측" /> }
        ]
    },
    {
        contract: "SignalLog",
        selectors: [".trdr-signal-list", ".trdr-signal", ".trdr-signal b", ".trdr-signal span", ".trdr-signal:last-child"],
        cases: [
            {
                state: "ready",
                element: (
                    <SignalLog
                        label="신호 기록"
                        signals={SIGNALS}
                        signalId={(record) => record.id}
                        signal={signalContent}
                        onInspect={() => undefined}
                    />
                )
            },
            {
                state: "loading",
                element: <SignalLog label="신호 기록" signals={[]} signalId={(record) => record.id} signal={signalContent} state="loading" />
            },
            {
                state: "empty",
                element: (
                    <SignalLog
                        label="신호 기록"
                        signals={[]}
                        signalId={(record) => record.id}
                        signal={signalContent}
                        state="empty"
                        empty={
                            <EmptyState
                                title="아직 신호가 없습니다."
                                explanation="등록된 규칙이 처음 일치하는 날 여기에 기록됩니다."
                            />
                        }
                    />
                )
            },
            {
                state: "error",
                element: <SignalLog label="신호 기록" signals={[]} signalId={(record) => record.id} signal={signalContent} state="error" />
            }
        ]
    },
    {
        contract: "InlineFeedback",
        selectors: [".trdr-inline-feedback"],
        cases: [
            {
                state: "visible",
                element: <InlineFeedback tone="error" message="연결 실패 · 설정에서 다시 연결하세요." onRetry={() => undefined} retryLabel="다시 시도" />,
                selectors: ['.trdr-inline-feedback[data-tone="error"]']
            },
            {
                state: "dismissed",
                element: <InlineFeedback tone="success" message="동기화 완료" state="dismissed" />,
                absent: true
            }
        ]
    },
    {
        contract: "EmptyState",
        selectors: [".trdr-empty-state", ".trdr-empty-state strong", ".trdr-empty-state p"],
        cases: [{ state: "empty", element: EMPTY }]
    }
];

export { EVENTS, HOLDINGS, RULES, SIGNALS, STRATEGY, eventContent, signalContent };
export type { Disclosure, Holding, SignalRecord };
