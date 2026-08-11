/**
 * The contract's recipes, actually composed.
 *
 * `contracts.v2.json` lists which roles each approved view is made of. These are
 * those lists, built — the proof that the package is finished is not that every
 * component exists but that the five structured views of Phase 1/2 can be put
 * together out of them without a single rule of their own. `recipes.test.tsx`
 * renders each one and fails on any class name the shipped stylesheet does not
 * define.
 *
 * Two recipes are absent on purpose. `PersistentAgentRail` is the terminal,
 * which the agent track owns, and `DurableMutation` is the approval round trip,
 * which is M6.
 *
 * Data is the synthetic fixture from the catalogue. Everything a screen would
 * fetch is a prop here, because this package reads nothing.
 */

import {
    Button,
    CoverageCard,
    DataTable,
    DayProgress,
    EventList,
    InlineFeedback,
    Metric,
    Panel,
    RuleGrid,
    SignalLog,
    StrategyCard,
    Tag,
    TopBar,
    ViewLead
} from "../index";
import {
    EVENTS,
    HOLDINGS,
    HOLDING_COLUMNS,
    RULES,
    SIGNALS,
    STRATEGY,
    eventContent,
    signalContent
} from "./catalogue";

/** The four numbers the account is read through. */
function AccountMetrics()
{
    return (
        <div className="trdr-metric-grid">
            <Panel>
                <Metric label="총 평가금액" value="28,307,940원" comparison="합성 데이터" />
            </Panel>
            <Panel>
                <Metric label="총손익" value="+786,300원" comparison="+2.85%" marketTone="up" />
            </Panel>
            <Panel>
                <Metric label="오늘 손익" value="−41,200원" comparison="−0.15%" marketTone="down" />
            </Panel>
            <Panel>
                <Metric label="예수금" value="1,204,000원" comparison="주문 가능 없음" />
            </Panel>
        </div>
    );
}

export function TodayView()
{
    return (
        <>
            <TopBar title="Today" context="합성 데이터" metadata="14:31 기준 · 로컬 보관" avatar="RY" />
            <AccountMetrics />

            <Panel title="보유 종목" caption="KIS 읽기 전용" end={<Tag tone="success" label="연결됨" />}>
                <DataTable label="보유 종목" columns={HOLDING_COLUMNS} rows={HOLDINGS} rowId={(row) => row.id} />
            </Panel>

            <Panel title="오늘의 이벤트" caption="계좌와 관련된 것만">
                <EventList label="오늘의 이벤트" items={EVENTS} itemId={(item) => item.id} event={eventContent} />
            </Panel>

            <Button variant="primary" label="실험실 열기" />
        </>
    );
}

export function LabDraftView()
{
    return (
        <>
            <TopBar title="Lab" context="초안" metadata="합성 데이터" />
            <ViewLead
                pill="Agent가 생성한 초안"
                headline="규칙을 재현 가능한 형식으로 고정합니다."
                description="등록하면 검증이 끝날 때까지 규칙은 바뀌지 않습니다."
                status={<Tag tone="focus" label="초안 v1" />}
            />

            <Panel title="실행 규칙" caption="strategy.yaml">
                <RuleGrid {...RULES} state="draft" />
            </Panel>

            <CoverageCard
                source="KIS 일봉"
                days={1258}
                covered={1190}
                total={1258}
                summary="1,190 / 1,258일 · 94.6% · 68일 결측"
                state="incomplete"
            />

            <Button variant="primary" label="백테스트 실행" />
        </>
    );
}

export function LabResultView()
{
    return (
        <>
            <TopBar title="Lab" context="결과 · low-vol-v1" metadata="입력 해시 8f21c4" />
            <ViewLead
                headline="검증 구간에서 기준을 넘었습니다."
                description="학습과 검증 구간의 차이를 먼저 확인하세요."
                status={<Tag tone="success" label="검증 통과" />}
            />

            <div className="trdr-metric-grid">
                <Panel>
                    <Metric label="검증 수익률" value="+8.4%" comparison="학습 +15.7%" marketTone="up" />
                </Panel>
                <Panel>
                    <Metric label="최대 낙폭" value="−7.8%" comparison="기준 −12.3%" />
                </Panel>
                <Panel>
                    <Metric label="승률" value="54.2%" comparison="168회 거래" />
                </Panel>
                <Panel>
                    <Metric label="초과수익" value="—" comparison="기준 지수 미확보" state="unavailable" />
                </Panel>
            </div>

            <Panel title="누적 수익 곡선" caption="검증 구간">
                <p className="trdr-muted">차트는 M4에서 이 자리에 들어갑니다.</p>
            </Panel>

            <Button variant="primary" label="모의검증에 등록" />
        </>
    );
}

export function StrategiesView()
{
    return (
        <>
            <TopBar title="Strategies" context="모의검증" metadata="합성 데이터" />
            <ViewLead
                headline="등록된 전략은 관측이 끝날 때까지 규칙이 고정됩니다."
                description="수익률보다 관측 진행과 규칙 이탈을 먼저 봅니다."
            />

            <InlineFeedback tone="warning" message="NAVER가 보유 조건에서 벗어났습니다." />

            <div className="trdr-strategy-list">
                <StrategyCard strategy={STRATEGY} href="#/strategies/low-vol-v1" />
                <StrategyCard
                    strategy={{
                        ...STRATEGY,
                        id: "mean-rev-v2",
                        name: "코스닥 평균 회귀",
                        status: <Tag label="검증 완료" />,
                        observation: { label: "관측", value: "20 / 20일", note: "100%" },
                        paperReturn: { label: "모의 수익률", value: "−1.4%", note: "등록 이후" },
                        currentState: { label: "상태", value: "이탈 0" }
                    }}
                    state="completed"
                    href="#/strategies/mean-rev-v2"
                />
            </div>
        </>
    );
}

export function StrategyDetailView()
{
    return (
        <>
            <TopBar
                title="대형주 저변동 모멘텀"
                context="low-vol-v1"
                metadata="마지막 평가 14:31"
                back={{ label: "전략 목록", onActivate: () => undefined }}
            />

            <div className="trdr-metric-grid">
                <Panel>
                    <Metric label="관측" value="8 / 20일" comparison="40%" />
                </Panel>
                <Panel>
                    <Metric label="모의 수익률" value="+2.1%" comparison="등록 이후" />
                </Panel>
                <Panel>
                    <Metric label="규칙 이탈" value="1" comparison="NAVER" />
                </Panel>
                <Panel>
                    <Metric label="신호" value="2" comparison="오늘" />
                </Panel>
            </div>

            <Panel title="관측 진행" caption="20 거래일">
                <DayProgress elapsed={8} total={20} today={8} label="20일 중 8일 관측" />
            </Panel>

            <Panel title="등록 규칙" caption="고정됨" end={<Tag tone="success" label="frozen" />}>
                <RuleGrid {...RULES} state="frozen" />
            </Panel>

            <Panel title="보유 구성" caption="일간 평가">
                <DataTable label="보유 구성" columns={HOLDING_COLUMNS} rows={HOLDINGS} rowId={(row) => row.id} />
            </Panel>

            <Panel title="신호 기록" caption="등록 규칙에 따른 것만">
                <SignalLog
                    label="신호 기록"
                    signals={SIGNALS}
                    signalId={(record) => record.id}
                    signal={signalContent}
                />
            </Panel>
        </>
    );
}

export const RECIPES =
{
    TodayView,
    LabDraftView,
    LabResultView,
    StrategiesView,
    StrategyDetailView
} as const;
