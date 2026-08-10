# trdr 구현 기초 설계안

> 상태: 구현 기준 v1 · 확정 2026-08-10
>
> 상위 계획: `docs/IMPLEMENTATION_PLAN.md`
>
> 이 문서는 첫 Today → Lab → 모의검증 production path를 구현하기 전에
> 프로세스, 권한, 저장, ingest, IPC와 도메인 불변식을 고정한다. 실제 코드,
> SQL DDL, source별 전체 endpoint 목록과 미래 plugin/cloud 구조는 정의하지 않는다.

## 1. 설계 목표

trdr는 사용자 관점에서는 하나의 macOS 앱이지만 내부에서는 다음 책임을 분리한다.

1. React UI는 화면과 사용자 의도만 다룬다.
2. Tauri/Rust host만 OS 권한, Keychain, 네트워크, DB writer와 PTY를 가진다.
3. Rust core는 UI와 OS에서 독립된 전략·데이터·검증 규칙의 정본이다.
4. `trdr` CLI는 같은 core를 사용하되, 실행 중 앱의 UI와 durable mutation은
   Unix socket을 통해 요청한다.
5. 공식 KIS·OpenDART·ECOS collector와 사용자 bundle은 같은 ingest validator와
   transaction을 통과한다.
6. KRX는 공식 collector·credential·설정·지원에 존재하지 않는다.
7. Agent 출력은 raw PTY byte stream으로만 보이며 제품 상태로 해석하지 않는다.

기초 설계의 성공은 폴더와 trait 수가 아니라, 아래 첫 경로가 중복 권한이나
우회 쓰기 없이 하나의 흐름으로 설명되는가로 판단한다.

```text
KIS 연결 → Today 확인 → 로컬 데이터 ingest → 스펙 검증 → 백테스트
→ 사람 승인 → 모의검증 등록 → Strategies에서 관측
```

## 2. 좁은 리뷰 디스패치 계약

### 필수 동작

- WebView는 Keychain, SQLite, 임의 파일, PTY process와 upstream API에 직접 접근하지 않는다.
- 공식 collector 세 개와 외부 bundle이 동일한 canonical ingest path를 사용한다.
- 외부 collector는 어떤 언어로든 작성할 수 있으나 앱 안에서 실행되지 않고 DB를 직접 쓰지 않는다.
- 앱과 CLI가 같은 DB를 사용해도 writer는 동시에 하나뿐이다.
- read-only CLI/UI 동작과 durable mutation의 승인 경계가 method 단위로 고정된다.
- 전략 등록은 실행 중 앱과 사람 승인이 없으면 성립하지 않는다.
- 같은 normalized spec, data snapshot, cost model, engine version은 같은 output hash를 만든다.
- 모든 오류는 안정된 code를 가지며 secret·계좌 원문·터미널 원문을 로그에 넣지 않는다.

### 변경되는 production surface

- `apps/desktop`: React 화면, Tauri host, macOS native high-authority sheet
- `crates/trdr-core`: domain, ingest validation, deterministic engine, hashes
- `crates/trdr-runtime`: SQLite, built-in collectors, Keychain, PTY, app IPC
- `crates/trdr-cli`: inspect, ingest, backtest, UI request commands
- `packages/ui`: 승인된 v2 UI contract의 React binding
- `schemas`: ingest bundle과 strategy spec의 versioned JSON Schema
- 사용자 workspace와 application runtime directory

### 지원 입력

- Apple Silicon, macOS 14 이상, 한 명의 로컬 OS 사용자
- KIS 개인 계좌, OpenDART와 ECOS의 사용자 API key
- Claude Code 또는 Codex executable 하나
- schema `trdr.ingest/v1` bundle
- 명시 종목 목록의 한국 주식 일봉
- v0 제한 전략 스펙: long-only, MA20/60, 동일비중, 최대 보유 수,
  종가 신호 후 다음 거래일 시가, 고정 비용

### 신뢰 경계

신뢰하는 것은 공식 서명 앱, 동일 OS 사용자와 함께 설치된 공식 `trdr` CLI다.
WebView 입력, Agent output, strategy file, ingest bundle, upstream response는 검증 대상이다.
같은 OS 사용자의 악성 process, root/admin, 손상된 운영체제 방어는 v0 밖이다.

### 명시적 제외

- 주문·broker mutation·자동매매
- 공식 KRX integration
- in-process third-party plugin과 plugin UI
- cloud/backend/PostgreSQL/Redis/Docker
- 일반 자연어 실행기
- 멀티유저·멀티디바이스 동기화
- v0 incremental/deduplicated/cloud backup과 Time Machine orchestration
- 전체 SQL DDL과 미래 시장·broker 추상화

### 충분한 증거의 중단 조건

설계 리뷰는 다음 질문에 문서만으로 한 가지 답이 나오면 끝난다.

1. 각 process는 무엇을 읽고 쓸 수 있는가?
2. secret은 어느 순간에도 WebView/CLI/로그/DB로 나오지 않는가?
3. 공식/사용자 데이터는 어디서 검증되고 언제 atomic commit되는가?
4. 앱과 CLI 중 누가 DB writer인가?
5. Agent 요청이 어떻게 사람 승인으로 바뀌고 같은 session으로 돌아가는가?
6. 어떤 input이 첫 엔진에서 지원되고 무엇이 거절되는가?

미래 plugin, cloud, 두 번째 broker, 이론적 공격 입력은 이 review의 blocking 범위가 아니다.
현재 경계 안의 secret 노출, 임의 쓰기, 데이터 손상과 결과 비결정성은 blocking이다.

## 3. 프로세스 모델

```text
┌──────────────────────────── trdr.app ─────────────────────────────┐
│                                                                  │
│  WKWebView process                                               │
│  React UI · xterm rendering                                      │
│      │                                                            │
│      │ typed Tauri commands/events                               │
│      ▼                                                            │
│  Tauri core process                                              │
│  AppController · QueryService · JobCoordinator · ApprovalBroker  │
│      ├─ native credential/approval sheet                          │
│      ├─ Keychain                                                  │
│      ├─ single SQLite writer                                      │
│      ├─ KIS · OpenDART · ECOS direct HTTPS                       │
│      ├─ Unix socket server                                        │
│      └─ PTY owner ───────────────┐                                │
└──────────────────────────────────┼────────────────────────────────┘
                                   ▼
                         Claude Code / Codex child
                                   │ raw bytes only
                                   ▼
                                xterm.js

external trdr CLI ── Unix socket ──► running app
       │
       ├─ read-only core/query when app is closed
       └─ exclusive writer lock only when app is closed

user collector ── writes bundle directory ──► trdr data ingest
```

Tauri의 system WebView와 core는 별도 보안 경계로 취급한다. UI가 앱과 같은 팀에서
작성됐다는 이유로 OS 권한을 직접 주지 않는다.

### 3.1 권한 표

| process | 허용 | 금지 |
|---|---|---|
| React/WebView | query model 표시, 의도 event, xterm byte 렌더링 | Keychain, DB, filesystem, HTTP, process spawn |
| Tauri host | OS 권한, validation orchestration, single writer, native sheet | 전략 규칙 자체를 UI별로 재구현 |
| Rust core | 순수 validation, normalization, backtest, hash, state transition | Tauri, Keychain, WebView, network 전용 API |
| Runtime | repository, HTTP adapter, credential handle, PTY, IPC | UI 문구 생성, Agent output 해석 |
| `trdr` CLI | core read/compute, bundle dry-run, app request | key 값 조회, 직접 registration, 앱 실행 중 직접 DB write |
| Agent child | terminal I/O와 `trdr` CLI 호출 | 앱 내부 권한, 승인, credential 변경 |
| user collector | 외부 source 호출, bundle 생성 | 앱 process 로드, SQLite 직접 접근, `trdr.*` namespace |

## 4. 저장소 구조와 의존 방향

```text
apps/
  desktop/
    src/                    React application
    src-tauri/              Tauri composition root, capabilities, native bridge
crates/
  trdr-core/                pure domain + ingest + engine + deterministic hashes
  trdr-runtime/             SQLite + collectors + Keychain + PTY + socket
  trdr-cli/                 CLI parser and presentation
packages/
  ui/                       tokens, reusable React components, view recipes
schemas/
  ingest/v1/
  strategy/v1/
fixtures/
  synthetic/
  redacted/
docs/
```

의존 방향은 한쪽이다.

```text
UI ──► typed command contract ──► runtime ──► core
CLI ─────────────────────────────► runtime ──► core
                                          └─► SQLite/HTTP/OS
```

- `trdr-core`는 Tauri와 React를 모른다.
- UI는 SQL row나 upstream response를 받지 않고 versioned query model만 받는다.
- runtime은 UI component나 한국어 문구를 모른다.
- app과 CLI에서 복제된 backtest/registration 규칙을 허용하지 않는다.
- 두 번째 구현이 생기기 전에는 collector별 crate, plugin SDK, cloud crate를 만들지 않는다.

## 5. 사용자 workspace와 runtime state

### 5.1 위치

고정 제품 루트는 `~/.trdr`다. 첫 실행은 `~/.trdr/workspaces/default`를 기본 workspace로
만들고, 사용자가 설정에서 다른 local workspace를 선택할 수 있게 한다. v0는 local filesystem만
지원하며 network mount와 동기화 중인 remote volume은 거절한다.

```text
~/.trdr/
  config.json               selected workspace path/id + local instance id, secret 없음
  run/
    app.sock                running-app Unix socket
    writer.lock             single writer lease
  workspaces/
    default/                기본 workspace
```

제품 루트와 `run`은 mode `0700`, 상태 파일은 `0600`으로 생성한다. 앱은
`Finder에서 데이터 폴더 열기`를 제공해 숨김 디렉터리의 가시성을 보완한다.

```text
<workspace>/
  workspace.json            portable workspace id, schema version, created-at
  trdr.sqlite3              canonical state
  trdr.sqlite3-wal          SQLite가 관리
  trdr.sqlite3-shm          SQLite가 관리
  .trdr/recovery/           자동 pre-migration recovery point
  objects/sha256/           immutable public/reference raw objects
  strategies/               user-owned .trdr.yaml files
  imports/                  user-staged ingest bundles
  exports/                  explicit exports and reports
  logs/                     redacted local logs
```

KIS·OpenDART·ECOS key와 token은 workspace에 없다. Keychain item은
`service=com.fxylabs.trdr`, account=`<local-instance-id>:<collector>:<field>`로 구분한다.
`workspace-id`는 백업과 함께 이동하는 논리 identity지만 `local-instance-id`는 한 Mac에서
workspace를 연결할 때 `~/.trdr/config.json`에 생성한다. 따라서 복구본이나 다른 Mac이 기존
Keychain secret을 이름 충돌로 자동 상속하지 않는다.

### 5.2 raw 보존 규칙

- OpenDART·ECOS와 KIS public market data의 수집 원문은 content-addressed object로 보존할 수 있다.
- KIS account raw response, access token, account number 원문은 object store에 넣지 않는다.
- account는 normalized latest snapshot만 저장하고 이전 snapshot은 v0에서 보존하지 않는다.
- 모든 persisted object는 SHA-256과 source manifest를 가진다.
- DB가 열린 상태에서 `trdr.sqlite3`, `-wal`, `-shm`을 Finder나 `cp`로 복사하는 것은
  지원되는 백업이 아니다. backup command가 SQLite snapshot과 파일 manifest를 함께 만든다.

## 6. SQLite 소유권과 transaction 규칙

앱과 CLI가 동시에 SQLite writer가 되지 않는다.

저장 엔진 결정은 다음과 같다. v0의 정본은 SQLite이고 일봉도 먼저 SQLite에 저장한다.
Parquet/DuckDB는 대표 데이터 benchmark에서 전체 스캔·백테스트 입력 준비가 실제 병목으로
확인될 때만 immutable 분석 snapshot과 보조 query engine으로 추가한다. PostgreSQL은
cloud·멀티유저·다중 writer 요구가 생기기 전에는 도입하지 않는다.

```text
앱 실행 중
  app owns writer.lock
  CLI read request ── socket/query
  CLI write request ── socket/job ── app transaction

앱 종료
  CLI may acquire writer.lock
  CLI ingest/backtest write ── one transaction
  strategy registration ── REFUSE (사람 승인 앱 필요)
```

기본 규칙:

- system SQLite를 사용하지 않고 SQLite 3.51.3 이상을 Rust binary에 고정해 번들한다.
- WAL mode, foreign keys on, busy timeout과 startup integrity check를 사용한다.
- write job 하나마다 명시적 transaction 하나를 사용한다.
- app 시작 시 writer lock과 socket의 stale 상태를 확인하고 PID만 믿지 않는다.
- migration은 app/CLI 둘 다 실행할 수 있지만 writer lock을 획득한 경우에만 한다.
- schema downgrade는 지원하지 않는다. 새 binary가 migration 전에 recovery point를 만든다.
- append-only entity는 SQL update/delete path를 repository API에 노출하지 않는다.
- SQLite connection이나 SQL은 WebView에 노출하지 않는다.

### 6.1 스키마 마이그레이션

DB schema, ingest bundle, backup package는 서로 다른 version을 가진다. 앱 업데이트는 DB schema만
자동 변경하며 사용자의 strategy file과 immutable raw object를 제자리에서 다시 쓰지 않는다.

```text
writer lease 획득
→ read-only header/application id/schema 검사
→ PRAGMA quick_check
→ pre-migration recovery point 생성·검증
→ 지원되는 ordered migration을 한 transaction에서 실행
→ migration checksum 기록
→ foreign_key_check + quick_check
→ commit
→ 앱 command 수락
```

- `schema_migrations`는 migration id, checksum, app version, applied time을 기록한다.
- DB `application_id`와 schema version이 예상과 다르거나 더 새 버전이면 쓰기 모드로 열지 않는다.
- migration file checksum이 설치 binary와 다르면 drift로 거절한다.
- 실패하면 transaction을 rollback하고 이전 DB를 그대로 유지한다. 자동으로 원본 파일을 덮어쓰는
  복구는 하지 않는다.
- SQLite page size나 file format을 바꾸는 비transactional migration은 v0에서 허용하지 않는다.
- 이전 앱으로 downgrade하지 않는다. 필요하면 pre-migration recovery point와 이전 앱을 명시적으로 쓴다.

### 6.2 백업 종류와 포맷

백업은 목적별로 두 종류다.

| 종류 | 목적 | 위치·보안 | 포함 범위 |
|---|---|---|---|
| local recovery point | schema migration 실패 직전으로 되돌리기 | workspace 내부, 사용자 전용 권한 | DB snapshot, workspace metadata, file manifest |
| portable full backup | 다른 디스크·Mac으로 이동 및 재해 복구 | 사용자 선택 위치, 기본 암호화 | DB snapshot, 참조 raw objects, strategy files, provenance manifest |

portable backup의 논리 구조는 versioned `*.trdrbackup` package다.

```text
manifest.json               backup/package/schema/app version, workspace id
database/trdr.sqlite3       SQLite Online Backup API로 만든 일관된 snapshot
objects/sha256/...          snapshot이 참조하는 immutable object만
strategies/...              backup 시점의 사용자 strategy file
checksums.json              package file path, size, SHA-256
```

- Online Backup API를 사용하며 live WAL 파일을 직접 복사하지 않는다.
- backup job은 새 write job을 잠시 막고 진행 중 transaction을 끝낸 뒤 snapshot barrier를 세운다.
- package는 임시 경로에 만들고 DB integrity, object reference, file hash를 검증한 뒤 최종 이름으로
  atomic rename한다. 중단된 `.partial`은 성공한 backup으로 표시하지 않는다.
- portable backup에는 Keychain key/token, runtime socket/lock, log, terminal byte, export, staged import,
  KIS account raw response가 들어가지 않는다.
- normalized account snapshot은 포함되므로 portable backup은 민감한 파일이다. 기본은 사용자
  passphrase와 검토된 암호화 library/format으로 보호하며 자체 암호 알고리즘은 만들지 않는다.
- local recovery point는 portable backup이 아니며 같은 디스크 장애를 막지 못한다.
- 자동 recovery retention은 수동 portable backup을 삭제하지 않는다.
- v0 portable backup은 full snapshot만 지원한다. incremental chain과 자체 cloud backup은 만들지 않는다.

### 6.3 복구와 다른 Mac으로 이전

복구는 live workspace에 파일을 덮어쓰지 않는다.

```text
package 인증/복호화
→ path traversal·symlink·size limit 검사
→ 모든 hash와 manifest 검사
→ snapshot DB integrity/application id/schema 검사
→ 새 temporary workspace에 materialize
→ 필요한 forward migration을 복구본에만 실행
→ domain/object-reference 검사
→ 새 local instance id 생성
→ 사용자에게 복구 요약 표시
→ 승인 후 selected-workspace pointer 전환
```

- 현재 workspace는 수정하거나 다시 백업하지 않고 그대로 둔다.
- 원본 backup은 절대 수정하지 않는다. 실패한 temporary workspace만 격리하거나 삭제 후보로 표시한다.
- 복구가 성공해도 credential 상태는 모두 `missing`이다. 새 Mac에서 사용자가 다시 입력한다.
- 동일한 portable `workspace-id`를 가진 두 경로를 한 Mac에서 동시에 활성화하지 않는다.
- 현재 binary에 명시된 migration path가 없는 오래되거나 더 새로운 backup은 거절한다.
- 전환 뒤 문제가 생기면 이전 workspace pointer로 되돌릴 수 있다. 복구본을 확인하기 전에는
  이전 workspace를 자동 삭제하지 않는다.

### 6.4 데이터 소유권용 export

backup은 trdr 전체 상태를 복원하기 위한 내부 package이고, export는 제품을 떠나서도 읽을 수 있는
데이터 이동 경로다. 둘을 같은 기능으로 취급하지 않는다.

- `trdr data export`는 선택한 canonical entity를 versioned NDJSON bundle로 만든다.
- raw object와 provenance를 선택적으로 포함하되 credential/account raw는 포함하지 않는다.
- export bundle은 새 workspace의 canonical ingest path로 다시 넣을 수 있다.
- 앱 schema가 바뀌어도 기존 export를 제자리에서 바꾸지 않고 지원 adapter로 읽거나 명시적으로
  새 버전 bundle을 생성한다.

## 7. 최소 도메인 모델

정확한 SQL table보다 먼저 정본 entity와 불변식을 정한다.

| entity | 역할 | 핵심 불변식 |
|---|---|---|
| `Workspace` | 한 사용자의 local root | stable id, local path |
| `Source` | `trdr.*` 또는 `user.*` data source | namespace와 provenance 필수 |
| `CollectionRun` | 한 번의 공식/외부 수집 | 상태·범위·collector version·hash |
| `RawObject` | immutable upstream object | content hash로 식별, account raw 금지 |
| `Instrument` | 종목 identity | source와 effective interval 명시 |
| `DailyBar` | 일봉 관측 | market/symbol/date natural key, revision append |
| `Disclosure` | OpenDART 공시 | receipt id, published/available time |
| `FinancialFact` | 지원 재무 fact | period, filing, available time, revision |
| `MacroSeries` | ECOS series metadata | stat/item/frequency identity |
| `MacroObservation` | ECOS 시계열 값 | period, available time, revision |
| `AccountSnapshot` | KIS latest normalized account | latest only, raw/token 없음 |
| `StrategyDraft` | user file의 현재 valid projection | file path + normalized hash |
| `DataSnapshot` | 한 run이 읽은 immutable record set | exact manifest hash |
| `BacktestRun` | deterministic execution | input/output hash, status, metrics |
| `Registration` | 사람 승인으로 고정된 전략 | append-only, spec/data/cost/engine hash |
| `ValidationEvent` | 모의검증의 일별 사실 | append-only, registration과 market date |

### 7.1 시간 필드

- `observed_at`: collector가 값을 본 시각
- `available_at`: 전략이 그 값을 합법적으로 사용할 수 있었던 시각
- `effective_at`: 사실이 시장/기업에 적용되는 시각
- `recorded_at`: trdr가 transaction을 commit한 시각

백테스트는 `available_at` 이후의 데이터만 읽는다. 네 필드를 하나의 `date`로 합치지 않는다.

### 7.2 hash 경계

```text
input_hash = SHA-256(
  normalized_strategy_spec
  + data_snapshot_manifest
  + execution_assumptions
  + cost_model
  + engine_version
)

output_hash = SHA-256(
  normalized_metrics
  + ordered_trades
  + ordered_equity_curve
  + input_hash
)
```

JSON object key와 record 순서, decimal·datetime 표현을 canonical form으로 고정한다.
금액은 KRW integer, 비율은 decimal string/고정 scale로 표현하고 binary float를 hash하지 않는다.

## 8. canonical ingest 계약

### 8.1 bundle 구조

```text
my-bundle/
  bundle.json
  records/
    daily_bar.ndjson
    macro_observation.ndjson
```

`bundle.json` 최소 형태:

```json
{
  "schema_version": "trdr.ingest/v1",
  "bundle_id": "018f...",
  "source": {
    "id": "user.example",
    "upstream_url": "https://example.test",
    "terms_url": "https://example.test/terms",
    "use_basis": "user-declared"
  },
  "collector": {
    "name": "example-collector",
    "version": "1.0.0"
  },
  "collected_at": "2026-08-10T10:00:00Z",
  "files": [
    {
      "entity": "daily_bar",
      "path": "records/daily_bar.ndjson",
      "records": 120,
      "sha256": "..."
    }
  ]
}
```

NDJSON record envelope:

```json
{
  "entity": "daily_bar",
  "key": {"market":"KR", "symbol":"005930", "date":"2026-08-07"},
  "observed_at": "2026-08-10T10:00:00Z",
  "available_at": "2026-08-08T00:00:00+09:00",
  "revision": 1,
  "payload": {"open":"70000", "high":"71000", "low":"69500", "close":"70500", "volume":"123456"}
}
```

### 8.2 ingest 순서

```text
discover bundle
→ path/symlink/size limits
→ manifest schema
→ file hash/count
→ record envelope schema
→ entity semantic validation
→ natural-key/revision conflict plan
→ dry-run report
→ staging
→ one atomic commit
→ collection run + snapshot manifest
```

- `--dry-run`은 DB를 바꾸지 않는다.
- unknown schema version, entity, field는 조용히 버리지 않고 거절한다.
- 같은 source/key/revision/payload는 idempotent no-op다.
- 같은 source/key/revision에 다른 payload가 오면 conflict다.
- 더 높은 revision은 이전 행을 지우지 않고 새 사실로 추가한다.
- 외부 source는 `user.*`, built-in source는 reserved `trdr.kis`, `trdr.opendart`,
  `trdr.ecos` namespace를 사용한다.
- `use_basis`는 사용자의 선언이지 trdr의 법률 검증 표시가 아니다.

### 8.3 collector 경계

내장 collector의 논리 interface:

```text
metadata() -> id, version, supported entities, credential fields
test(credential_handle) -> connected | invalid | rate_limited | unavailable
plan(cursor, requested_scope) -> bounded requests
collect(plan, credential_handle) -> validated bundle stream + next cursor
```

- `credential_handle`은 secret string을 UI/CLI에 반환하지 않는다.
- 수집기는 raw response를 canonical row로 직접 commit하지 않고 ingest service에 넘긴다.
- cursor는 성공한 transaction 이후에만 전진한다.
- KIS account snapshot은 민감 데이터 전용 normalizer로 들어가며 public ingest bundle로 export하지 않는다.
- user collector는 이 Rust interface를 구현하지 않는다. 표준 bundle만 만들면 된다.
- KRX-specific code, 로그인 필드와 도움말은 내장 collector registry에 없다.

## 9. 두 IPC의 계약

### 9.1 React WebView ↔ Tauri host

Tauri command는 broad filesystem/shell/database plugin 권한을 UI에 열지 않고 앱이 소유한
좁은 command만 등록한다.

초기 read commands:

```text
bootstrap.get
today.get
collectors.list
lab.draft.get
backtest.get
strategies.list
strategy.get
jobs.get
backup.list
```

초기 intent commands:

```text
collector.credential.open_native_sheet
collector.test
collector.collect
workspace.choose
ingest.dry_run
ingest.commit
backtest.run
backup.create
backup.verify
workspace.restore.open_native_sheet
approval.respond
terminal.start | terminal.input | terminal.resize | terminal.restart
```

- command parameter와 return은 versioned JSON type이다.
- UI는 path를 임의 문자열로 넘기지 않고 native picker가 만든 scoped handle을 넘긴다.
- Tauri capability는 main WebView 하나에 필요한 command만 explicit enable한다.
- shell, filesystem, SQL의 generic Tauri plugin command는 WebView에 노출하지 않는다.
- Rust → UI event는 job/connection/process 상태 변화에만 쓰고 terminal byte stream과 분리한다.

### 9.2 `trdr` CLI ↔ running app

Unix socket protocol은 newline-delimited JSON request/response다.

```json
{"v":1,"id":"...","method":"ui.open","params":{"resource":"strategy","id":"low-vol-v1"}}
{"v":1,"id":"...","ok":true,"result":{"status":"opened"}}
```

- socket directory와 file은 사용자 전용 권한이다.
- host는 macOS peer uid를 확인한다.
- protocol version과 request id는 필수다.
- unknown method/field/version은 거절한다.
- read request는 bounded response만 돌려준다. secret/raw account payload는 없다.
- app이 없으면 UI command와 durable mutation은 `APP_NOT_RUNNING`이다.
- CLI는 human text와 `--format json`을 같은 structured result에서 렌더링한다.

초기 method:

```text
app.status
ui.open
ui.focus
account.inspect
data.coverage
ingest.request
backtest.run
backtest.inspect
backup.create.request
backup.verify
workspace.restore.request
strategy.inspect
strategy.register.request
```

### 9.3 registration 승인 왕복

```text
CLI validates request
→ host creates ApprovalRequest(request_id, exact command, hashes, effects, expiry)
→ native app sheet takes focus
→ user approves / rejects / request expires
→ host revalidates current hashes
→ one registration transaction or no write
→ same socket request receives terminal result
```

- 한 번에 registration approval 하나만 표시한다.
- initial focus는 취소에 둔다.
- 요청 중 CLI가 끊기면 request를 취소한다.
- approve 직전 spec/data hash가 바뀌면 `STALE_APPROVAL`로 거절한다.
- duplicate request id는 같은 terminal result를 돌려준다.
- UI, CLI, Agent 어디에도 approval 우회 method가 없다.

## 10. high-authority native surface

React가 대부분의 앱 UI를 담당하지만 네 표면은 macOS native sheet로 둔다.

1. KIS·OpenDART·ECOS credential 입력/갱신
2. portable backup passphrase 입력
3. registration과 향후 durable mutation 승인
4. workspace 복구·전환 승인

credential sheet는 secure text field에서 Keychain으로 직접 저장하고 WebView에는
`missing | untested | valid | invalid` 상태만 반환한다. approval sheet는 exact command,
외부 효과, local effect, 고정 hash와 만료를 표시한다.

기초 spike에서 Rust/AppKit bridge 방법을 결정한다. 전체 SwiftUI 앱이나 독립 Swift 상태 모델은
만들지 않는다. native sheet도 Rust core의 validation result를 받아 표시할 뿐 규칙을 재구현하지 않는다.

## 11. 화면용 query model

UI는 upstream/DB 모델을 직접 조합하지 않는다. Rust query service가 화면별 versioned model을 만든다.

| query | 입력 정본 | 출력 요약 |
|---|---|---|
| `TodayModel/v1` | latest account snapshot, holding disclosures, validation events | 총액, 보유, 오늘 관련 event, stale state |
| `LabDraftModel/v1` | valid strategy draft, coverage | 규칙, 기간, 지원 여부, missing data |
| `LabResultModel/v1` | backtest run + snapshot | metrics, curve, assumptions, hashes, warnings |
| `StrategiesModel/v1` | registrations + derived latest events | 진행/완료, 관측일, 참고 성적, rule deviation |
| `StrategyDetailModel/v1` | one registration + events | frozen rules, paper positions, signal log, lineage |

- 화면 문구는 finite state와 code를 React localization table에 매핑한다.
- LLM이 headline, verdict, error text를 작성하지 않는다.
- market color, connection color, focus color 역할은 UI contract를 그대로 따른다.
- raw terminal content와 product query model은 어떤 방향으로도 변환하지 않는다.

## 12. 상태와 오류

기존 UI contract의 async, broker, terminal, approval, paper-validation state를 정본으로 사용한다.
foundation에서 추가하는 job state는 다음뿐이다.

```text
IngestJob: planned → validating → ready → committing → committed | rejected
CollectorJob: queued → requesting → normalizing → ingesting → completed | partial | failed
BacktestJob: queued → validating → running → completed | cancelled | failed
BackupJob: queued → snapshotting → packaging → verifying → completed | failed
RestoreJob: queued → authenticating → verifying → materializing → ready → switched | rejected
Credential 상태는 missing → untested → valid | invalid | rate-limited 순서로 전이한다.
```

안정된 error code family:

```text
AUTH_*          missing, invalid, rate-limited
UPSTREAM_*      unavailable, timeout, malformed
BUNDLE_*        schema, hash, size, conflict
DATA_*          incomplete, unsupported, future-leak, integrity
STRATEGY_*      syntax, unsupported-rule, invalid-period
DB_*            busy, migration, integrity, disk-full
BACKUP_*        authentication, checksum, unsupported-version, incomplete
APP_*           not-running, protocol-version, permission
APPROVAL_*      rejected, expired, stale, disconnected
TERMINAL_*      executable-missing, spawn, exited
```

- error에는 code, safe message parameters, retryability, cause chain id만 노출한다.
- HTTP header/body, credential, account number, terminal bytes는 structured log field가 될 수 없다.
- UI와 CLI는 같은 error code를 각 표면에 맞게 렌더링한다.

## 13. 테스트 seam

core/runtime은 다음 외부 효과를 interface 뒤에 둔다.

```text
Clock
IdGenerator
CredentialStore
HttpTransport
FileSystem
WriterLease
Repository
PtyHost
AppBridge
BackupCodec
```

필수 증거:

| 경계 | 자동 테스트 | 실제 smoke |
|---|---|---|
| core | golden, property, canonical hash, look-ahead | 손계산 fixture 대조 |
| ingest | malformed/oversize/symlink/conflict/rollback | 사용자 collector bundle 1개 |
| SQLite | migration, crash transaction, writer lease | app+CLI 경쟁 시나리오 |
| backup/restore | interrupted backup, hash/schema/path, migration rollback | 다른 Mac용 package 왕복 |
| credential | canary non-disclosure | 세 source 실제 key test |
| collector | mock HTTP, rate-limit, cursor retry | KIS/OpenDART/ECOS 각 최소 1건 |
| IPC | version, unknown method, disconnect, duplicate | Claude/Codex session 각 1회 |
| approval | approve/reject/expire/stale/crash | native sheet 전체 왕복 |
| UI | component states, query-model fixtures | 1440×900·1080p visual QA |

테스트 fixture에는 real API key, account number, 실제 계좌 payload를 commit하지 않는다.

## 14. 첫 scaffold가 만들어야 하는 것

기초 설계 승인 뒤 첫 scaffold work는 다음만 만든다.

1. pnpm/Cargo workspace와 네 production package
2. Tauri main window와 explicit capability file
3. React AppShell에 Today/Lab/Strategies route와 persistent empty terminal host
4. `trdr-core`의 workspace id, error envelope, command/query type
5. bundled patched SQLite 연결, migration 0, writer lease
6. `trdr-cli app status`와 Unix socket ping
7. synthetic fixture 한 개와 CI의 format/lint/test

이 단계에서는 real KIS call, full DB schema, backtest, registration, collector 구현을 하지 않는다.
scaffold의 stop condition은 app과 CLI가 같은 workspace/runtime을 찾고, WebView가 generic OS 권한 없이
typed ping/query를 왕복하며, 앱이 single writer lease를 가진다는 증거다.

## 15. 확정된 scaffold 기본값

1. 제품 루트 `~/.trdr`, 기본 workspace `~/.trdr/workspaces/default`
   (`01kznnr5x818p3j6enyksadp8w`)
2. bundle id와 Keychain service `com.fxylabs.trdr`
   (`01kznp0gq3x8ark9dq489z7wj8`)
3. credential, backup passphrase, registration approval, workspace restore/switch만
   AppKit native sheet (`01kznp0kqt4ak72j624etkegyk`)
4. `trdr-core`, `trdr-runtime`, `trdr-cli` runtime split
   (`01kznp0sx24hrgsx2as8hy5fzh`)
5. 첫 scaffold는 synthetic path까지만 만들고 실제 API/엔진은 다음 work로 분리
   (`01kznp0ybajp3dqb1rb8qbk5wy`)
6. Online Backup API 기반 recovery point, 기본 암호화 portable full backup,
   새 workspace 복구 (`01kznp0zwphrc9k7wcj97qwpa6`)

foundation의 scaffold 전 결정 게이트는 모두 닫혔다. 구현은 상위 계획의 M0 정본화 이후
이 문서의 section 14 범위로 시작한다.
