# trdr OSS macOS 구현 계획

> 상태: 승인된 구현 계획 v1 · 2026-08-10
>
> 목표: 확정된 local-first OSS 방향을 실제 제품으로 옮기기 위한 좁은 실행 계획.
> 이 문서는 구현을 시작하지 않으며, 기존 목업·구형 PRD의 충돌을 먼저 닫고
> 가장 작은 Today → Lab → 모의검증 수직 슬라이스를 순서대로 정의한다.

## 1. 첫 검증 릴리스의 완료 상태

첫 외부 검증 릴리스는 다음 한 문장으로 정의한다.

> Apple Silicon Mac 사용자가 공식 공증 DMG를 설치하고, KIS 개인 계정을
> 읽기 전용으로 연결해 Today를 확인한 뒤, 자기 로컬 일봉 데이터로 제한된
> 전략 하나를 결정론적으로 백테스트하고, 네이티브 승인창을 거쳐 20거래일
> 모의검증에 등록할 수 있다. API 키·계좌·전략·데이터는 Mac 밖으로 나가지 않는다.

공식 내장 수집기는 KIS, OpenDART, ECOS다. 사용자가 자기 API 키로 자기 Mac에서
직접 수집하며, 세 수집기와 외부 사용자 수집기는 하나의 canonical ingest 계약을
통과한다. KRX 수집기는 공식 코드·설정·지원 경로에서 제외한다.

제품 코드는 UI까지 AGPLv3로 공개한다. `trdr` 상표, 공식 Apple Developer ID,
릴리스 서명 키와 업데이트 채널은 프로젝트가 관리한다. 제3자는 자기 이름과
서명으로 포크를 배포할 수 있다.

첫 릴리스에 실제 주문은 없다. Agent가 생성한 자연어를 앱이 곧바로 실행하는
경로도 없다. Agent는 제한된 전략 스펙을 작성하고 CLI를 호출할 뿐이며, 앱과
CLI는 동일한 검증·백테스트 코어를 사용한다.

## 2. 좁은 리뷰 디스패치 계약

### 필수 동작

1. 공식 앱은 macOS에서 일반 `.app`/DMG로 설치되고 Developer ID 서명과 Apple
   공증을 검증할 수 있다.
2. KIS·OpenDART·ECOS 자격증명은 Keychain에만 저장되고 UI, 터미널, 로그,
   SQLite, crash report에 평문으로 나타나지 않는다.
3. 앱은 KIS 개인 계좌의 잔고·보유종목을 직접 읽어 Today에 표시한다. 주문 API는
   호출하지 않는다.
4. KIS, OpenDART, ECOS 수집기는 사용자 자격증명으로 로컬에서 직접 실행되고,
   외부 사용자 수집기와 동일한 validation·provenance·commit 경로를 사용한다.
5. 사용자가 만든 외부 수집 결과는 versioned bundle과 `trdr data ingest`를 통해서만
   들어온다. 외부 코드가 내부 SQLite를 직접 쓰거나 앱 프로세스 안에서 실행되지 않는다.
6. 사용자가 고른 로컬 일봉 데이터와 제한된 전략 스펙으로 같은 입력이 항상 같은
   결과 해시를 만든다.
7. Lab은 규칙, 데이터 커버리지, 비용 가정, 학습/검증 분리, 결과와 근거 해시를
   보여준다.
8. 전략 등록은 CLI 요청만으로 완료되지 않는다. 실행 중인 앱의 네이티브 승인창에서
   사람이 승인해야 하며, 승인 결과는 같은 CLI 세션으로 돌아간다.
9. Strategies는 고정된 규칙, 관측 진행, 모의 포지션, 규칙 이탈과 평가 이력을
   로컬 데이터에서 재계산해 보여준다.
10. Claude Code 또는 Codex의 원시 PTY 출력과 scrollback은 Today/Lab/Strategies
   이동 중에도 유지된다. 앱은 출력을 카드나 의미 데이터로 재해석하지 않는다.

### 변경되는 production surface

- 루트 OSS·기여·보안·상표 문서와 패키지 메타데이터
- Tauri macOS 앱 셸과 React UI
- Rust 도메인 코어, SQLite 저장소, 결정론 백테스트
- `trdr` CLI와 실행 중 앱 사이의 로컬 IPC
- Keychain 및 KIS read-only account/market-data adapter
- OpenDART disclosure/financial collector와 ECOS macro-statistics collector
- 외부 수집기용 versioned ingest bundle, CLI와 source-rights manifest
- macOS 서명·공증·DMG 릴리스 절차

### v0 지원 입력

- Apple Silicon, macOS 14 이상, 한 명의 로컬 OS 사용자
- 이미 설치되고 인증된 Claude Code 또는 Codex 실행 파일 하나
- KIS 개인 계좌용 app key/secret, OpenDART API key, ECOS API key
- KIS read-only 계좌·시세, OpenDART 공시·기업/재무정보, ECOS 통계표·시계열 관측값
- 사용자가 직접 고른 한국 상장 보통주 종목 목록
- canonical CSV 또는 상업 이용이 허용된 소스에서 사용자 기기로 직접 수집한 일봉
  OHLCV 데이터
- long-only, 현금+주식, 공매도·레버리지 없음
- MA20/MA60 교차, 최대 보유 종목 수, 동일 비중, 일간 평가
- 종가 신호 후 다음 거래일 시가 체결, 고정 수수료와 슬리피지 가정
- 고정된 학습/검증 구간과 20거래일 모의검증

이 범위는 엔진의 영구 한계가 아니라 첫 수직 슬라이스의 검증 가능한 계약이다.
명시되지 않은 전략 연산자는 지원하지 않고 오류로 거절한다.

### 신뢰 경계

신뢰하는 것은 공식 서명 앱·동일 OS 사용자·로컬 `trdr` CLI다. Agent의 터미널 출력,
전략 파일, ingest bundle과 KIS/OpenDART/ECOS 응답은 모두 검증이 필요한 입력이다.

- 자격증명을 읽는 권한은 데스크톱 host에만 둔다. Agent와 CLI는 키 값을 읽을 API가 없다.
- CLI의 UI 조작은 사용자 전용 Unix socket과 per-install 인증 토큰을 거친다.
- socket 권한은 같은 OS 사용자로 제한하고, durable mutation은 앱이 떠 있지 않으면 거절한다.
- SQLite와 파일은 사용자 전용 권한으로 저장한다. 앱 수준 암호화를 주장하지 않으며,
  root/admin 권한 또는 같은 계정의 악성 프로세스 방어는 v0 위협 모델 밖이다.
- telemetry는 기본 off다. 켜더라도 API 키, 계좌값, 종목 목록, 전략, 터미널 내용은 수집하지 않는다.

### 명시적 제외

- 실주문, 주문 preview, broker mutation, 자동매매, copy trading
- Toss 및 두 번째 broker, 공식 KRX 수집기·자격증명 UI·지원, 제한 데이터 중앙 재배포
- 번들된 실시장 데이터, 유료 데이터 라이선스, 역사적 지수 구성종목 보장
- 분봉·틱·실시간 백테스트, 파생상품, 공매도, 레버리지, 세금 최적화
- 일반 자연어 전략 실행기, 전략 marketplace, plugin UI
- cloud sync, 계정 서버, 원격 Agent, hosted runner
- App Store, Intel Mac, Windows, Linux, mobile
- 과거 dockable-pane 모델과 자유 레이아웃

### 충분한 증거의 중단 조건

구현 리뷰는 아래 증거가 모두 생기면 멈춘다.

1. 깨끗한 Apple Silicon macOS 사용자 계정에서 공증 DMG 설치·첫 실행 통과
2. 실제 KIS 개인 계정으로 read-only Today smoke 통과, 주문 endpoint 호출 0건
3. golden fixture의 동일 input manifest가 반복 실행과 CLI/UI에서 동일 output hash 생성
4. Agent 요청 → 앱 승인/거절/만료 → 같은 CLI 세션 복귀 E2E 통과
5. 1440×900과 1080p에서 승인된 UI contract의 상태가 screen-local CSS 없이 렌더링
6. secret canary가 UI, 로그, SQLite, export, crash artifact에 나타나지 않음
7. 외부 타깃 사용자 5명 이상이 설치 → Today → Lab → 등록을 개발자 도움 없이 완주

이 범위 밖의 hardening과 미래 입력은 블로킹 리뷰 항목으로 확장하지 않는다. 단,
현재 신뢰 경계 안의 보안·개인정보·데이터 권리·결과 무결성 문제는 차단 사유다.

## 3. 권장 아키텍처

사용자에게는 macOS 네이티브 앱이지만 UI는 기존 HTML/CSS 시스템을 재사용한다.

```text
┌────────────────────── official macOS app ──────────────────────┐
│ React/TypeScript UI                                             │
│ Today · Lab · Strategies · raw xterm                            │
│             │ Tauri commands/events                            │
│ Tauri host ──┼── Keychain ── direct KIS read-only              │
│             │             ├── direct OpenDART                  │
│             │             └── direct ECOS                      │
│             ├── PTY ── Claude Code or Codex                    │
│             ├── local IPC server ── approval/UI requests       │
│             └── trdr-core                                      │
└──────────────────────────┬──────────────────────────────────────┘
                           │ SQLite / local files
        canonical data · specs · runs · registrations
                           ▲
              validated ingest bundle / same Rust core
                  user collector ── trdr CLI
```

### 기술 선택

| 영역 | 선택 | 이유 |
|---|---|---|
| 앱 셸 | Tauri 2 | 기존 웹 UI 재사용, macOS 번들·서명·공증, Rust host |
| UI | React + TypeScript + Vite | 상태가 많은 고정 데스크톱 UI와 xterm 생태계, 기존 CSS contract 이식 |
| 터미널 | xterm.js + OS PTY | Agent CLI 원형 보존, scrollback 지속 |
| 도메인 | Rust library | 앱·CLI에서 같은 validation/backtest/hash 사용 |
| 저장 | SQLite migrations + append-only event tables | 로컬 소유, 재현성, 간단한 백업 |
| 자격증명 | macOS Keychain | 키 값 비노출 계약 |
| 앱 연결 | 사용자 전용 Unix domain socket | `trdr ui`와 승인 요청의 좁은 로컬 bridge |
| 릴리스 | Developer ID 서명·Apple 공증 DMG | App Store sandbox 없이 공식 provenance 제공 |

### 최소 저장소 구조

```text
apps/desktop/          React UI + Tauri host
crates/trdr-core/      domain, schema, ingest, store, connectors, engine modules
crates/trdr-cli/       inspect, backtest, ui, registration request commands
packages/ui/           v2 tokens/components/contracts의 production binding
schemas/ingest/        versioned manifest와 entity JSON Schemas
fixtures/              synthetic/redacted deterministic fixtures
docs/                  current contracts, rights registry, release evidence
```

미래 plugin SDK, cloud package, broker별 독립 crate 등은 실제 두 번째 구현이 생길 때까지
만들지 않는다.

### 권한 분리

| 행위 | UI | CLI/Agent | 자동 |
|---|---:|---:|---:|
| 캐시된 상태 읽기 | 허용 | 허용 | 허용 |
| 백테스트 실행 | 허용 | 허용 | 허용 |
| structured view 열기 | 허용 | 허용 | — |
| KIS 키 등록·변경 | 사람만 | 거절 | 거절 |
| KIS 계좌 새로고침 | 허용 | 키 비노출 요청만 허용 | 앱 실행 중만 |
| KIS·OpenDART·ECOS 수집 | 허용 | 수집 요청만 허용 | 앱 실행 중만 |
| 외부 bundle dry-run/ingest | 허용 | 허용 | 거절 |
| 전략 등록 | 승인자 | 요청만 | 거절 |
| 등록 수정·삭제 | 거절 | 거절 | 거절 |
| 주문 | 코드 경로 자체 없음 | 코드 경로 자체 없음 | 코드 경로 자체 없음 |

## 4. 구현 순서와 게이트

### M0. 정본 계약과 OSS 기반 정리

목표: 서로 충돌하는 구형 문서를 코딩 입력에서 제거한다.

- 최신 고정 3화면(Today/Lab/Strategies), persistent raw terminal, 네이티브 승인 흐름을
  PRD와 MVP 범위의 정본으로 승격한다.
- Electron, dockview, 자유 pane, KRX 1999~ 수급, user-zero만의 인수 기준, live-order 암시를
  현재 v0에서 제거하거나 명시적으로 보관 문서로 표시한다.
- Tauri 2 결정을 확정하고 macOS 14+/Apple Silicon 범위를 기록한다.
- 공식 내장 local collector는 KIS·OpenDART·ECOS, 공식 제외 source는 KRX로 기록한다.
- 외부 수집기는 임의 plugin runtime이 아니라 versioned ingest bundle 경계로 고정한다.
- `LICENSE`(AGPLv3), `TRADEMARKS.md`, `SECURITY.md`, `PRIVACY.md`,
  `CONTRIBUTING.md`, third-party notice 정책을 추가한다.
- `package.json`의 `UNLICENSED`/`private` 상태를 공개 전환 계획과 맞춘다.
- `CONTRIBUTING.md`에는 초기 외부 코드 기여와 PR을 받지 않으며, 버그·제안과 프로젝트
  작업은 이슈를 기준으로 처리한다고 명시한다.
- 외부 기여를 열기 전에 DCO/CLA, contributor licensing, PR/merge 정책과 향후
  dual-license 여부를 별도로 결정한다.
- 기존 stale mockup work는 최신 UI kit과의 계보를 확인해 완료 또는 retire한다.

증거 게이트: PRD, MVP, UI contract, CLI contract 사이에 실행 경로 충돌이 없고,
의존성 라이선스가 AGPL 배포와 충돌하지 않는다.

### M1. 세 가지 위험 스파이크

제품 UI를 넓게 구현하기 전에 실패 비용이 큰 세 경로만 실행한다.

1. **PTY spike**: 사용자가 선택한 Claude/Codex executable을 spawn하고, raw ANSI·입력·resize·
   scrollback·route 이동 후 지속성을 증명한다. GUI 앱이 shell의 `$PATH`를 상속한다고 가정하지 않는다.
2. **Keychain/KIS spike**: canary key를 저장·갱신·삭제하고 mock KIS 서버에 인증 요청을 보낸다.
   모든 로그와 DB에서 canary가 검출되지 않아야 한다.
3. **CLI bridge spike**: `trdr ui open`, read-only inspect, registration approval request를 Unix socket으로
   보내고 approve/reject/expire가 같은 CLI 프로세스로 돌아오는 것을 증명한다.

중단 조건: 셋 중 하나가 신뢰 경계를 깨거나 Tauri에서 안정적으로 구현되지 않으면 기능 개발을
계속하지 않고 그 경로 또는 앱 셸 선택을 다시 결정한다.

### M2. fixture 기반 walking skeleton

목표: 승인된 UI kit을 실제 앱 안에서 한 번 끝까지 실행한다.

- AppShell, Sidebar, Today, Lab draft/result, Strategies/list/detail, persistent terminal을 구현한다.
- UI는 `tokens.v2`와 `contracts.v2`를 production component로 옮기며 screen-local 시각 규칙을 만들지 않는다.
- SQLite 최초 migration과 application-support 경로를 만든다.
- 합성 fixture로 loading/ready/empty/stale/error와 terminal process 7상태를 재현한다.
- UI와 CLI가 같은 fixture domain object를 읽게 한다.
- 계좌·시장 데이터가 아닌 합성 데이터임을 항상 표시한다.

증거 게이트: 앱 재시작 후 route와 PTY가 계약대로 동작하고, 모든 승인된 화면이 실제 Tauri 앱에서
1440×900 및 1080p 시각 검수를 통과한다.

### M3. 표준 ingest, 내장 local collector와 KIS read-only Today

목표: 모든 데이터가 같은 검증 경로로 들어오게 하고 local-first 신뢰 명제를 첫 실제 데이터로 증명한다.

#### 표준 insert 경로

- 외부 수집기는 `bundle.json` manifest와 entity별 `records.ndjson`를 만든다.
- `trdr data ingest <bundle> --dry-run`은 schema, 크기, natural key, 시간, 중복,
  source namespace와 provenance를 검사하고 DB를 바꾸지 않는다.
- `trdr data ingest <bundle>`은 staging → validation → 한 transaction commit 순서로 실행한다.
- manifest는 schema version, source id, collector name/version, collected-at, data window,
  upstream/terms URL, 사용자가 선언한 이용 범위, raw/records hash를 가진다.
- record envelope는 entity type, natural key, observed-at, available-at, revision과 payload를 가진다.
- 첫 entity type은 `instrument`, `daily_bar`, `disclosure`, `financial_fact`,
  `macro_series`, `macro_observation`으로 제한한다.
- source id의 `trdr.*` namespace는 내장 collector만 사용한다. 외부 수집기는 `user.*`를 사용한다.
- idempotency key가 같은 record는 재실행해도 중복되지 않는다. 수정 관측값은 revision으로 추가하며
  기존 사실을 조용히 덮어쓰지 않는다.
- 외부 수집기는 어떤 언어로든 만들 수 있지만 trdr 프로세스 안에서 실행되지 않고 SQLite에 직접 접근하지 않는다.
- KRX 자료도 사용자가 적법하게 확보했다면 `user.*` bundle로 넣을 수 있다. trdr는 KRX collector,
  로그인, 자격증명 필드, source-specific 지원을 제공하지 않는다.

#### 내장 collector

| collector | 공식 범위 | 자격증명·실행 |
|---|---|---|
| KIS | read-only 계좌, 보유종목, 국내주식 시세·일봉 | Keychain, 사용자 Mac에서 direct call |
| OpenDART | corp code, 공시 목록/원문 참조, 지원 재무 fact | Keychain, 사용자 Mac에서 direct call |
| ECOS | 통계표 metadata와 사용자가 선택한 거시 시계열 관측값 | Keychain, 사용자 Mac에서 direct call |

세 내장 collector도 별도 우회 insert를 만들지 않고 동일한 ingest validator와 transaction을 호출한다.

- 설정/온보딩에 KIS key 입력, 테스트 호출, 연결 상태, key 갱신을 추가한다.
- 같은 수집기 표면에서 OpenDART·ECOS key 입력, 테스트 호출, 수집 범위와 마지막 성공 시각을 보여준다.
- 허용된 read-only endpoint 목록만 가진 KIS client를 구현한다. 주문 endpoint client는 만들지 않는다.
- OpenDART와 ECOS는 명시된 read endpoint만 구현하고 rate limit, empty, revision 상태를 구분한다.
- 잔고·총평가·보유종목·당일 손익을 정규화해 Today에 표시한다.
- 마지막 성공 snapshot 하나만 로컬에 보존하고, stale/disconnected/error 상태와 시각을 표시한다.
- 요청/응답 로그는 field allowlist 방식으로 남기고 계좌번호·token·secret을 redaction한다.
- 실제 호출은 사용자의 계정으로 수동 smoke하고, 자동 테스트는 redacted fixture/mock server를 사용한다.

증거 게이트: built-in과 external fixture bundle이 같은 canonical row/hash를 만들고, malformed bundle은
원자적으로 거절된다. 실제 개인 계정에서 Today 값을 KIS 원본과 대조하며 broker mutation 호출은 0건이다.
OpenDART 공시 1건과 ECOS 시계열 1개를 실제 key로 수집하고, 앱 종료·재실행 후에도 secret canary가 남지 않는다.

### M4. 로컬 데이터와 결정론 엔진

목표: 라이선스 구매나 중앙 데이터 재배포 없이 믿을 수 있는 첫 백테스트를 만든다.

- instrument, daily bar, source, snapshot manifest, strategy spec, run, registration, signal event schema를 만든다.
- canonical CSV importer를 표준 ingest bundle을 만드는 첫 reference adapter로 구현한다.
- KIS·OpenDART·ECOS는 사용자 Mac에서 직접 호출하며, real data를 DMG에 넣지 않는다.
- source별 URL, 이용조건 snapshot, 수집시각, raw hash, transform version을 rights/provenance manifest에 기록한다.
- 첫 전략 연산자를 MA20/60 교차·동일비중·최대 종목 수로 제한한다.
- 다음 거래일 시가 체결과 비용 모델을 명시하고 look-ahead를 금지한다.
- input hash는 normalized spec, exact data manifest, cost model, engine version을 포함한다.
- 결측, 중복, 비거래일, 미해결 corporate action이 있으면 조용히 보정하지 않고 해당 run을 차단하거나
  결과에 명시된 지원 범위로 제한한다.

데이터 무결성 게이트: 역사적 종목 구성·상장폐지·수정주가가 확보되지 않은 상태에서는
“생존자 편향 없음”, “total return”, “KOSPI 200 대비”를 주장하거나 렌더링하지 않는다.

증거 게이트: 고정 fixture golden 결과, property tests, 반복/CLI/UI hash 일치, 미래 데이터 누출 검사,
coverage failure 검사가 통과한다.

### M5. Lab과 Agent/CLI 실제 연결

목표: 자연어 가설이 검증된 제한 스펙과 재현 결과로 바뀌는 루프를 닫는다.

- `.trdr.yaml`의 첫 JSON Schema와 사람이 읽는 오류를 정의한다.
- `trdr strategy validate`, `trdr backtest run`, `trdr account inspect --format json`,
  `trdr ui open|focus`를 구현한다.
- Agent는 파일을 쓰고 CLI를 호출한다. 앱 내부 LLM·자연어 parser는 만들지 않는다.
- 파일 변경은 validate 성공 후에만 Lab draft를 갱신한다.
- run 진행·취소·실패와 결과 화면을 구현한다.
- 결과는 규칙, coverage, 비용, 구간, provenance, input/output hash를 함께 보존한다.

증거 게이트: Claude Code와 Codex 각각 한 번씩 같은 전략 파일을 만들고 동일 CLI 경로로 실행해
같은 결과를 연다. 앱은 terminal bytes를 의미 데이터로 파싱하지 않는다.

### M6. 사람 승인 등록과 모의검증

목표: 백테스트 결과를 수정 불가능한 로컬 검증 기록으로 전환한다.

- 등록 요청에는 exact command, 효과, 외부 주문 없음, 고정되는 규칙/데이터/비용/기간을 표시한다.
- 앱이 떠 있지 않거나 request가 만료되면 등록을 거절한다.
- 승인 후 append-only registration과 event를 한 transaction으로 기록한다.
- 삭제·수정 API와 UI는 만들지 않는다. 후속 변경은 새 registration과 lineage로만 표현한다.
- 앱 실행/새로고침 시 누락 거래일을 로컬 데이터에서 backfill 평가한다. background daemon은 만들지 않는다.
- Strategies list/detail, 20일 진행, 현재 모의 포지션, 신호·규칙 이탈, 완료 상태를 구현한다.
- 종료 전 성적은 참고용으로 표시하고 완료 verdict와 혼동하지 않는다.

증거 게이트: approve/reject/expire/crash recovery, DB transaction interruption, duplicate request,
lineage와 append-only invariant 테스트가 통과한다.

### M7. OSS·공식 DMG 릴리스

목표: 코드 공개와 공식 binary provenance를 동시에 성립시킨다.

- source tag, dependency lockfile, third-party notices, SBOM, commit SHA를 고정한다.
- 첫 릴리스는 로컬 release Mac에서 수동으로 서명·공증한다. CI signing은 필요가 증명된 뒤 추가한다.
- DMG SHA-256, Apple Team ID 확인법, 공증 확인법, source tag를 release page에 공개한다.
- 공식 updater가 들어가면 별도 update signing key로 manifest를 검증하고, 제3자 fork feed와 섞이지 않게 한다.
- clean macOS user account에서 install, first launch, Keychain prompt, uninstall/data-retention을 검증한다.

증거 게이트: Gatekeeper가 개발자를 확인하고 공증 ticket을 검증하며, 배포 파일 hash와 source tag가
release evidence에 남는다.

### M8. 제품 가치 검증

목표: OSS 관심이 아니라 실제 반복 사용과 유료 가능성을 판정한다.

- 개발자가 아닌 목표 사용자 5~10명을 직접 모집한다.
- 측정 경로: DMG 설치 → KIS read-only 연결 → Today 확인 → 로컬 데이터 준비 → 첫 Lab run → 등록.
- activation은 전 과정을 도움 없이 완주한 사용자로 센다.
- retention은 2주 안에 Today 또는 Strategies를 다시 연 사용자로 센다.
- 인터뷰에서 local data/API-key ownership이 선택 이유였는지 확인한다.
- 별도로 유료 onboarding, 우선 지원 또는 검증된 자동 데이터 업데이트에 실제 결제를 제안한다.
- GitHub star, clone, issue 수는 discovery 신호로만 기록한다.

첫 go/no-go 기준:

- 10명 중 5명 이상 독립 activation
- 3명 이상 2주 내 반복 사용
- 2명 이상 실제 유료 제안 수락

충족하지 않으면 데이터 라이선스나 기능 범위를 늘리지 않는다. 가장 많이 막힌 한 단계만 수정해
같은 cohort 크기로 다시 검증한다.

## 5. 테스트와 증거 매트릭스

| 위험 | 자동 증거 | 수동 증거 |
|---|---|---|
| secret 노출 | canary scan, redaction unit/integration | Keychain·로그·DB·crash artifact 검사 |
| broker mutation | endpoint allowlist test | KIS 요청 기록에서 order 호출 0 확인 |
| 백테스트 오류 | golden/property/determinism/look-ahead tests | 한 fixture 수계산 표본 대조 |
| 데이터 오해 | schema/coverage/provenance tests | source rights·price semantics 검토 |
| CLI/UI 분기 | shared-core hash test, IPC E2E | Claude/Codex 실제 session 각 1회 |
| 등록 무결성 | transaction/append-only/lineage tests | 승인·거절·만료·재실행 시나리오 |
| UI 회귀 | component/state screenshots | 1440×900·1080p visual QA |
| 배포 신뢰 | signature/notarization/hash checks | clean user account 설치 |

## 6. 작업 그래프 제안

계획 승인 후 다음 순서로 별도 work unit을 만든다. 아직 구현 work를 시작하지 않는다.

```text
I0 정본 계약·OSS 정책
 ├─ I1 PTY/Keychain/IPC 위험 스파이크
 │   └─ I2 fixture walking skeleton
 ├─ I3 standard ingest + KIS/OpenDART/ECOS + read-only Today
 └─ I4 canonical import + deterministic engine
      └─ I5 Lab + CLI/Agent loop
           └─ I6 approval + paper validation
                └─ I7 signed/notarized OSS release
                     └─ I8 external value validation
```

- `w-0vja9`와 `w-b2v8x`는 구형 plugin/pane 전제를 버리고 `w-vpeht`로 retire됐다.
- 완료된 `w-vpeht`의 `docs/FOUNDATION_DESIGN.md`를 process/storage/IPC 정본으로 사용한다.
- 완료된 `w-rmm0a`의 v2 UI contract를 I2의 시각 정본으로 사용한다.
- pane/dockview 계열의 구형 work는 superself에서 모두 retire됐다. I0는 기록을 다시 정리하지 않고
  repository의 구형 문서만 최신 fixed shell과 맞춘다.

## 7. 예상 규모

한 명이 전념한다는 가정의 순서 규모이며 약속 일정은 아니다.

| 구간 | 대략 |
|---|---:|
| M0 계약 정리 | 2~3일 |
| M1 위험 스파이크 | 3~5일 |
| M2 앱 walking skeleton | 5~7일 |
| M3 ingest·내장 수집기·KIS Today | 8~12일 |
| M4 데이터·엔진 | 8~12일 |
| M5 Lab·CLI | 5~8일 |
| M6 모의검증 | 5~8일 |
| M7 공식 릴리스 | 3~5일 |
| M8 외부 검증 | 최소 2~4주 관찰 |

KIS 실제 응답, 데이터 price semantics, PTY/IPC 스파이크가 가장 큰 변동 요인이다.
M1과 M4 게이트 전에는 뒤 단계 일정을 확정하지 않는다.

## 8. 구현 시작 전 기본값

1~10은 foundation 설계와 사용자 결정으로 확정됐다.

1. Tauri 2 + React/TypeScript/Vite + Rust core/CLI
2. Apple Silicon, macOS 14 이상만 v0 지원
3. 고정 Today/Lab/Strategies와 420px raw terminal rail
4. 실데이터 번들 없음: 합성 demo + canonical ingest bundle + 사용자 기기 직접 수집
5. 첫 전략은 MA20/60 long-only 일봉 하나
6. 공식 내장 collector는 KIS·OpenDART·ECOS, KRX는 공식 경로에서 제외
7. broker는 KIS만 지원하고 read-only로 제한
8. 외부 수집기는 `user.*` versioned bundle을 `trdr data ingest`로 적재
9. 공식 DMG는 direct distribution, App Store 제외
10. 초기 OSS 단계에는 외부 코드 기여와 PR을 받지 않는다. 버그·제안 접수와 프로젝트
    변경 추적은 이슈를 기준으로 한다. 외부 기여를 열기 전 DCO/CLA와 contributor
    licensing 정책을 다시 결정한다.

구현 계획은 승인됐다 (`01kznpf5c9gmevwzt1paqkwerd`). 10의 정책 proposal
`01kznpf9a2wy1kyjchdpcd09pd`는 결정 `01kznpx0zk33kws99hpt3jq24c`로 대체됐다.
M0를 첫 구현 work로 등록하고, 이후 단계는 각 게이트의 증거가 생길 때만 하나씩
활성화한다.
