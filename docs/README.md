# trdr 문서 안내

이 저장소에서 어떤 질문에 어떤 문서가 답하는지 정리한다. 여기 없는 문서는 구현 입력이 아니다.

## 질문별 정본

| 질문 | 문서 |
|---|---|
| 무엇을 만들고, v0의 완료 상태는 무엇인가 | `docs/IMPLEMENTATION_PLAN.md` 1장 |
| 무엇이 지원 입력이고 무엇이 제외인가 | `docs/IMPLEMENTATION_PLAN.md` 2장 |
| 어떤 순서로 만들고, 각 단계의 증거 게이트는 무엇인가 | `docs/IMPLEMENTATION_PLAN.md` 4장 |
| 각 프로세스가 무엇을 읽고 쓸 수 있는가 | `docs/FOUNDATION_DESIGN.md` 3장 |
| 데이터가 어디에 저장되고 누가 쓰기를 갖는가 | `docs/FOUNDATION_DESIGN.md` 5~6장 |
| 외부 데이터가 어떤 계약으로 들어오는가 | `docs/FOUNDATION_DESIGN.md` 8장 |
| UI·CLI가 앱과 어떻게 통신하는가 | `docs/FOUNDATION_DESIGN.md` 9장 |
| 첫 scaffold가 만들어야 하는 것 | `docs/FOUNDATION_DESIGN.md` 14장 |
| 화면이 어떻게 보여야 하는가 | `artifacts/ui-kit/README.v2.md`, `contracts.v2.json`, `tokens.v2.json` |
| 라이선스·상표·보안·개인정보·기여 정책 | 저장소 루트의 `LICENSE`, `TRADEMARKS.md`, `SECURITY.md`, `PRIVACY.md`, `CONTRIBUTING.md`, `THIRD_PARTY_NOTICES.md` |
| 다음 구현 세션이 이어받을 상태 | `docs/IMPLEMENTATION_HANDOFF.md` |

제품 요구사항 정본은 별도 PRD가 아니라 `docs/IMPLEMENTATION_PLAN.md` 1~4장이다. 같은 내용을
두 문서가 각자 서술하면 다시 어긋나기 때문에 PRD를 새로 쓰지 않았다.

## 답이 서로 다를 때의 우선순위

1. `self context`와 `AGENTS.md`
2. `docs/FOUNDATION_DESIGN.md`
3. `docs/IMPLEMENTATION_PLAN.md`
4. `artifacts/ui-kit/README.v2.md`와 `contracts.v2.json`
5. `artifacts/ui-kit/tokens.v2.json`, `tokens.v2.css`, `components.v2.css`

## artifacts의 상태

- `artifacts/ui-kit/`가 시각 정본이다. v2 파일만 해당한다.
- 나머지 HTML·PNG는 그 정본이 나오기까지의 과정 기록이다. 구현이 참조하지 않는다.
- `artifacts/ui-kit/HANDOFF.v2.md`는 UI 시스템 작업이 끝나기 전에 쓰인 세션 인수 문서다.
  기록으로 보존하며 남은 작업 목록은 유효하지 않다.

## 삭제된 문서

2026-08-06~08-07에 쓰인 6건(`PRD.md`, `MVP_SCOPE.md`, `USER_JOURNEY.md`, `USER_STORIES.md`,
`SCREEN_ACTIONS.md`, `SCREEN_COMPOSITION.md`)을 2026-08-10에 삭제했다. 확정된 계약과 어긋난
채로 두면 다음 세션이 두 가지 답을 읽는다. 원문은 커밋 `797d396`의 `docs/`에 그대로 있고,
`git show 797d396:docs/PRD.md`로 읽을 수 있다.

무엇이 이 문서들을 무효화했는지만 여기 남긴다.

| 폐기된 전제 | 확정된 계약 |
|---|---|
| Electron + dockview 셸, pane 자유 배치와 프리셋 저장 | Tauri 2 고정 단일 창. Today/Lab/Strategies와 오른쪽 raw terminal rail |
| KRX 수급 이력을 공식 수집기로 제공 | 공식 collector는 KIS·OpenDART·ECOS. KRX 자료는 사용자가 확보해 `user.*` bundle로만 적재 |
| 앱 MCP 서버로 에이전트가 앱을 조작 | 에이전트는 파일과 `trdr` CLI만 쓴다. CLI는 Unix socket으로 앱에 요청한다 |
| 사용자 1명(user zero) 인수 기준 | 외부 목표 사용자 5~10명의 activation·retention·유료 수용 기준 |

문서 전체가 틀렸다는 뜻은 아니다. 아래 판단은 현재 계약에도 그대로 있다.

- 사전등록·검증·결과의 정직성 규율, 축하 연출 없는 결과 표시
- 절차의 무게는 시각으로, 용어는 일상어로
- 주문·실행은 v0에 존재하지 않는다
- 자격증명은 Keychain 전용, 값은 어디에도 노출하지 않는다
- 결과에는 출처 라벨과 재현 검증 해시가 함께 간다
