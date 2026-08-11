/**
 * Every word the window shows, in one table.
 *
 * Section 11 gives the rule this file exists to keep: a screen maps finite
 * states and stable codes to a localisation table, and no language model writes
 * a headline, a verdict, or an error text. So the models carry values —
 * `stale`, `qualified`, `DATA_INCOMPLETE` — and the wording for every one of
 * them is here, written once, by a person.
 *
 * Three consequences are worth naming.
 *
 * # The kit holds no copy, so the copy has to be somewhere
 *
 * `@trdr/ui` takes its labels as required props — `BrokerConnection.stateLabel`,
 * `DayProgress.label`, `EmptyState.explanation` — precisely so that the words
 * live in the application and the components stay reusable. This is where they
 * live. A Korean string typed into a screen is a string nobody can find again.
 *
 * # Exhaustiveness is the point of the `Record` types
 *
 * `Record<ErrorCode, string>` is not decoration: adding a variant to
 * `ErrorCode` in Rust regenerates `bindings.ts`, and the missing key here
 * becomes a compile error. The same holds for every state model below. A table
 * that can silently miss a case is a table that renders a blank where a person
 * needed a sentence.
 *
 * # Codes that are not enums still go through a table
 *
 * A backtest warning, a rule deviation, a signal action and a Today event
 * summary all arrive as `string` rather than as a closed enum, because the
 * runtime is allowed to grow the vocabulary without a breaking change. They are
 * still codes and never sentences, so each has a lookup below with a stated
 * fallback — the code itself, which is a thing a person can report, rather than
 * an empty line.
 */

import type {
    BrokerConnectionState,
    ErrorCode,
    MarketTone,
    PaperValidationState,
    SectionState,
    SupportState,
    TodayEventKind,
    Verdict
} from "../bindings";

/**
 * What the window says about the data it is showing.
 *
 * `statement` is the sentence milestone M2 requires to be true at all times,
 * and `label` is the same fact short enough for a top bar. Both are reached
 * only from a model's `origin` field, never from a constant, so the day the
 * data stops being synthetic the words stop with it.
 */
export const ORIGIN =
{
    synthetic:
    {
        label: "합성 데이터",
        statement: "합성 데이터입니다. 실제 계좌도, 실제 시장도, 실제 브로커도 아닙니다."
    },
    collected:
    {
        label: "수집한 데이터",
        statement: "이 워크스페이스에 설정된 소스에서 수집한 데이터입니다."
    }
} as const;

/** The three places Phase 1/2 navigates between, plus the two detail views. */
export const NAV =
{
    today: "오늘",
    lab: "실험실",
    strategies: "전략",
    strategyDetail: "전략 상세"
} as const;

/** The shell's own landmarks and the one line the status bar carries. */
export const SHELL =
{
    brand: "trdr",
    navigation: "구역",
    workspace: "작업 영역",
    agentRail: "에이전트 레일",
    broker: "KIS",
    brokerAuthority: "읽기 전용",
    workspaceLabel: "워크스페이스",
    versionLabel: "빌드",
    connecting: "호스트에 묻는 중…",
    unreachable: "호스트가 응답하지 않았습니다"
} as const;

/** A request that has not answered yet, and one that answered badly. */
export const REQUEST =
{
    loadingSection: "불러오는 중",
    failed: "화면을 채우지 못했습니다."
} as const;

/**
 * How much of a section is actually there.
 *
 * `stale` is the one that matters: it is a section that has its numbers and is
 * saying they are older than they should be, so the wording admits the age
 * rather than hiding the value.
 */
export const SECTION_STATE: Record<SectionState, string> =
{
    ready: "최신",
    empty: "표시할 내용 없음",
    stale: "지연된 값",
    error: "불러오지 못함"
};

/** The broker link. Green means connected, and never that money was made. */
export const BROKER_CONNECTION: Record<BrokerConnectionState, string> =
{
    "connected-read-only": "연결됨",
    syncing: "동기화 중",
    stale: "응답이 오래됨",
    disconnected: "연결 끊김",
    error: "연결 실패"
};

/** Where a paper validation has got to. `extended` is not a kind of running. */
export const PAPER_VALIDATION: Record<PaperValidationState, string> =
{
    draft: "초안",
    ready: "등록 완료",
    running: "모의검증 중",
    completed: "관측 완료",
    extended: "관측 기간 연장",
    discarded: "폐기"
};

/** Whether trdr will run a draft, and why not when it will not. */
export const SUPPORT: Record<SupportState, string> =
{
    supported: "실행 가능",
    "missing-data": "데이터 부족",
    "unsupported-rule": "미지원 규칙",
    "invalid-period": "잘못된 기간"
};

/** The Lab draft's one headline, chosen by the draft's own support state. */
export const SUPPORT_HEADLINE: Record<SupportState, string> =
{
    supported: "규칙과 기간이 모두 실행 가능한 상태입니다.",
    "missing-data": "기간의 일부 데이터가 없어 결과를 그대로 읽을 수 없습니다.",
    "unsupported-rule": "이 엔진이 실행하지 않는 규칙이 들어 있습니다.",
    "invalid-period": "검증 기간 자체를 실행할 수 없습니다."
};

/** trdr's reading of a backtest, as words rather than as a sentence it wrote. */
export const VERDICT: Record<Verdict, string> =
{
    sound: "가정 충족",
    qualified: "조건부",
    unsupported: "판단 불가"
};

/** The Lab result's one headline, chosen by the verdict. */
export const VERDICT_HEADLINE: Record<Verdict, string> =
{
    sound: "실행이 끝났고 가정이 모두 유지되었습니다.",
    qualified: "실행은 끝났지만 경고가 결과를 읽는 범위를 제한합니다.",
    unsupported: "뒷받침하는 데이터가 부족해 지표를 읽을 수 없습니다."
};

/** Which way a number moved. Never derived from a sign — the model says so. */
export const MARKET_TONE: Record<MarketTone, string> =
{
    up: "상승",
    down: "하락",
    flat: "보합"
};

/** The event's type, as the two or three characters that sit in its tile. */
export const TODAY_EVENT_KIND: Record<TodayEventKind, string> =
{
    disclosure: "공시",
    "strategy-signal": "전략",
    "validation-progress": "관측"
};

/**
 * Every failure section 12 defines, as a sentence a person can act on.
 *
 * The list is closed: section 12 fixes the families and `ErrorCode` fixes the
 * variants, so this table is complete by construction and stays complete
 * because the compiler says so.
 */
export const ERROR: Record<ErrorCode, string> =
{
    AUTH_MISSING: "이 소스에 저장된 자격증명이 없습니다. 설정에서 먼저 연결하세요.",
    AUTH_INVALID: "저장된 자격증명을 소스가 거부했습니다. 설정에서 다시 연결하세요.",
    AUTH_RATE_LIMITED: "소스가 당분간 요청을 더 받지 않습니다. 잠시 뒤 다시 시도하세요.",
    UPSTREAM_UNAVAILABLE: "소스에 닿지 못했습니다. 네트워크를 확인한 뒤 다시 시도하세요.",
    UPSTREAM_TIMEOUT: "소스가 제때 응답하지 않았습니다. 잠시 뒤 다시 시도하세요.",
    UPSTREAM_MALFORMED: "소스가 trdr이 읽을 수 없는 응답을 보냈습니다.",
    BUNDLE_SCHEMA: "번들이 스스로 선언한 스키마와 맞지 않습니다.",
    BUNDLE_HASH: "번들 파일이 기록된 해시와 다릅니다.",
    BUNDLE_SIZE: "번들이 허용된 크기를 넘었습니다.",
    BUNDLE_CONFLICT: "번들의 기록이 이미 저장된 기록과 어긋납니다.",
    DATA_INCOMPLETE: "요청한 범위를 데이터가 다 덮지 못합니다.",
    DATA_UNSUPPORTED: "이 버전이 다루지 않는 종류의 데이터입니다.",
    DATA_FUTURE_LEAK: "알 수 없었던 시점의 값이 쓰일 뻔했습니다.",
    DATA_INTEGRITY: "저장된 데이터가 자체 검사를 통과하지 못했습니다.",
    STRATEGY_SYNTAX: "전략 파일을 전략으로 읽지 못했습니다.",
    STRATEGY_UNSUPPORTED_RULE: "이 엔진이 실행하지 않는 규칙을 전략이 요구합니다.",
    STRATEGY_INVALID_PERIOD: "전략이 실행할 수 없는 기간을 지정했습니다.",
    DB_BUSY: "다른 쪽이 데이터베이스를 쥐고 있습니다. 잠시 뒤 다시 시도하세요.",
    DB_MIGRATION: "스키마 마이그레이션을 적용하지 못했습니다.",
    DB_INTEGRITY: "데이터베이스가 무결성 검사를 통과하지 못했습니다.",
    DB_DISK_FULL: "디스크에 남은 공간이 없습니다.",
    BACKUP_AUTHENTICATION: "백업 꾸러미를 인증하거나 복호화하지 못했습니다.",
    BACKUP_CHECKSUM: "백업 꾸러미가 체크섬 검사를 통과하지 못했습니다.",
    BACKUP_UNSUPPORTED_VERSION: "이 빌드가 읽을 수 없는 버전의 백업입니다.",
    BACKUP_INCOMPLETE: "백업 꾸러미의 일부가 빠져 있습니다.",
    APP_NOT_RUNNING: "앱 호스트가 응답하지 않아 이 화면을 채우지 못했습니다.",
    APP_PROTOCOL_VERSION: "응답이 프로토콜에 맞지 않습니다. 앱과 CLI 버전을 맞추세요.",
    APP_PERMISSION: "이 요청을 할 권한이 없습니다.",
    APPROVAL_REJECTED: "요청이 거절되었습니다.",
    APPROVAL_EXPIRED: "요청이 제때 응답되지 않았습니다.",
    APPROVAL_STALE: "승인한 내용이 지금 실행될 내용과 달라졌습니다.",
    APPROVAL_DISCONNECTED: "요청한 쪽이 답을 받기 전에 사라졌습니다.",
    TERMINAL_EXECUTABLE_MISSING: "에이전트 실행 파일이 예상한 자리에 없습니다.",
    TERMINAL_SPAWN: "에이전트 프로세스를 시작하지 못했습니다.",
    TERMINAL_EXITED: "에이전트 프로세스가 종료되었습니다."
};

/**
 * The vocabularies that arrive as `string` rather than as a closed enum.
 *
 * Each is a code the runtime writes down, never a sentence, and each has a
 * fallback that shows the code itself. A code on screen is ugly and reportable;
 * a blank line is neither.
 */
const OPEN_VOCABULARY: Record<string, string> =
{
    /* `TodayEvent::summary`, written by the query service. */
    "disclosure-for-holding": "보유 종목에 관한 공시가 도착했습니다.",
    "observation-day-recorded": "관측일 하나가 기록되었습니다.",

    /* `Signal::action`. */
    entry: "진입",
    exit: "청산",
    hold: "유지",

    /* `RuleDeviation::code` and `BacktestWarning::code` reuse section 12's
       families, so the sentences above already cover them; only the shorter
       label a table cell needs is added here. */
    "DATA_INCOMPLETE:short": "데이터 부족",
    "DATA_FUTURE_LEAK:short": "미래 참조",
    "STRATEGY_UNSUPPORTED_RULE:short": "미지원 규칙"
};

/** One open-vocabulary code in words, or the code itself when it is new. */
export function words(code: string): string
{
    return OPEN_VOCABULARY[code] ?? code;
}

/** The same, for a table cell that has no room for a sentence. */
export function shortWords(code: string): string
{
    return OPEN_VOCABULARY[`${code}:short`] ?? words(code);
}

/**
 * A backtest warning in words, with its safe parameters interpolated.
 *
 * `BacktestWarning` carries a code and a list of parameters and never a
 * sentence, exactly so that the wording is this file's decision. The parameters
 * are market dates and counts — section 12 forbids anything larger — so they
 * are appended rather than woven in, which keeps one sentence per code instead
 * of one per shape of parameter list.
 */
export function warningSentence(code: string, params: readonly string[]): string
{
    const sentence = ERROR[code as ErrorCode] ?? words(code);

    return params.length === 0 ? sentence : `${sentence} (${params.join(" ~ ")})`;
}

/** Everything the five screens name, in the order the screens read them. */
export const SCREEN =
{
    today:
    {
        title: "오늘",
        totalValue: "총 평가금액",
        dayChange: "오늘 손익",
        cash: "예수금",
        holdingCount: "보유 종목",
        holdings: "보유 종목",
        holdingsCaption: "브로커 읽기 전용",
        events: "오늘의 이벤트",
        eventsCaption: "계좌와 관련된 것만",
        openLab: "실험실 열기",
        columnSymbol: "종목",
        columnMarketValue: "평가금액",
        columnUnrealized: "평가손익",
        columnLastPrice: "현재가",
        holdingSummary: (symbol: string, quantity: string, average: string) =>
            `${symbol} · ${quantity} · 평균 ${average}`,
        emptyHoldings: "보유한 종목이 없습니다.",
        emptyHoldingsWhy: "브로커를 연결하고 계좌를 동기화하면 이 자리에 보유 종목이 들어옵니다.",
        emptyEvents: "오늘 기록된 이벤트가 없습니다.",
        emptyEventsWhy: "보유 종목 공시와 등록된 전략의 관측 기록이 생기면 여기에 쌓입니다."
    },
    labDraft:
    {
        title: "실험실",
        context: "초안",
        rules: "실행 규칙",
        period: (start: string, end: string) => `${start} ~ ${end}`,
        coverage: "데이터 커버리지",
        coverageSummary: (covered: number, total: number, percent: number) =>
            `${covered} / ${total}일 · ${percent}%`,
        coverageGaps: (ranges: string) => `결측 ${ranges}`,
        coverageNoGaps: "결측 없음",
        emptyCoverage: "커버리지를 계산할 소스가 없습니다.",
        emptyCoverageWhy: "초안이 참조하는 데이터 소스를 먼저 적재하세요.",
        seeResult: "백테스트 결과 보기",
        sectionUniverse: "대상과 순위",
        sectionEntry: "진입과 비중",
        sectionExitAndCost: "청산과 비용",
        sectionValidationWindow: "검증 분리",
        rule: (label: string, value: string) => `${label} · ${value}`
    },
    labResult:
    {
        title: "실험실",
        context: "결과",
        inputHash: (hash: string) => `입력 해시 ${hash}`,
        totalReturn: "총 수익률",
        annualised: (value: string) => `연환산 ${value}`,
        maxDrawdown: "최대 낙폭",
        winRate: "승률",
        trades: (count: number) => `${count}회 거래`,
        sharpe: "샤프",
        curve: "누적 수익 곡선",
        curveCaption: "차트는 M4에서 이 자리에 들어갑니다",
        curvePeriod: "기간",
        curveRange: (from: string, to: string) => `${from} ~ ${to}`,
        curvePoints: (points: number) => `${points}개 지점`,
        curveEquity: "시작 → 끝 평가금액",
        curveEnds: (first: string, last: string) => `${first} → ${last}`,
        curveEmpty: "곡선으로 그릴 지점이 없습니다.",
        curveEmptyWhy: "실행이 하루도 채우지 못했거나 곡선이 아직 저장되지 않았습니다.",
        assumptions: "실행 가정",
        assumptionsCaption: "결과를 읽기 전에 확인합니다",
        fill: "체결",
        commission: "수수료",
        slippage: "슬리피지",
        tax: "세금",
        engine: "엔진",
        outputHash: "출력 해시",
        register: "모의검증에 등록",
        registerNote: "등록 승인은 M6에서 열립니다."
    },
    strategies:
    {
        title: "전략",
        context: "모의검증",
        headline: "등록된 전략은 관측이 끝날 때까지 규칙이 고정됩니다.",
        description: "수익률보다 관측 진행과 규칙 이탈을 먼저 봅니다.",
        deviationsWarning: (count: number) => `등록 규칙과 다르게 실행된 기록이 ${count}건 있습니다.`,
        observation: "관측",
        observationValue: (elapsed: number, total: number) => `${elapsed} / ${total}일`,
        observationNote: (percent: number) => `${percent}%`,
        paperReturn: "모의 수익률",
        paperReturnNote: "등록 이후",
        currentState: "상태",
        deviationCount: (count: number) => (count === 0 ? "이탈 없음" : `이탈 ${count}`),
        remaining: (days: number) => `D−${days}`,
        empty: "아직 등록된 전략이 없습니다.",
        emptyWhy: "실험실에서 백테스트 결과를 확인한 뒤 모의검증에 등록할 수 있습니다."
    },
    strategyDetail:
    {
        observation: "관측",
        deviations: "규칙 이탈",
        signals: "신호",
        positions: "보유 구성",
        progress: "관측 진행",
        progressCaption: (total: number) => `${total} 거래일`,
        progressLabel: (elapsed: number, total: number) => `${total}일 중 ${elapsed}일 관측`,
        rules: "등록 규칙",
        rulesCaption: "고정됨",
        frozen: "고정",
        positionsCaption: "모의 보유 · 누구도 소유하지 않은 주식입니다",
        columnSymbol: "종목",
        columnQuantity: "수량",
        columnEntryPrice: "진입가",
        columnLastPrice: "현재가",
        positionSummary: (symbol: string, entered: string) => `${symbol} · ${entered} 진입`,
        emptyPositions: "모의 보유가 없습니다.",
        emptyPositionsWhy: "관측 중 진입 신호가 나오면 이 자리에 들어옵니다.",
        signalLog: "신호 기록",
        signalLogCaption: "등록 규칙에 따른 것만",
        signalTitle: (action: string, symbol: string) => `${action} · ${symbol}`,
        signalRule: (rule: string) => `규칙: ${rule}`,
        emptySignals: "기록된 신호가 없습니다.",
        emptySignalsWhy: "등록 규칙이 처음 맞아떨어지면 여기에 한 줄이 남습니다.",
        deviationLog: "규칙 이탈",
        deviationCaption: "등록 규칙과 다르게 실행된 기록",
        columnDate: "날짜",
        columnRule: "규칙",
        columnReason: "사유",
        emptyDeviations: "규칙과 다르게 실행된 기록이 없습니다.",
        emptyDeviationsWhy: "관측이 끝날 때까지 규칙은 고정되어 있습니다.",
        lineage: "등록 이력",
        lineageCaption: "등록은 덧붙기만 하고 고쳐지지 않습니다",
        registeredAt: "승인 시각",
        supersedes: "이전 등록",
        approvedRun: "승인 근거 실행",
        specHash: "규칙 해시",
        none: "없음",
        back: "전략"
    }
} as const;
