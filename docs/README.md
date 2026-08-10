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

## 보관 문서

아래 문서는 `docs/archive/`로 옮겼다. 본문은 원문 그대로이고, 각 파일 머리에 보관 사유와
대체 문서를 적었다. 자세한 내용은 `docs/archive/README.md`에 있다.

| 보관 문서 | 폐기된 전제 |
|---|---|
| `archive/PRD.md` | 자유 배치 pane 카탈로그, KRX 공식 수집 |
| `archive/MVP_SCOPE.md` | Electron + dockview 셸, 앱 MCP 서버 |
| `archive/USER_JOURNEY.md` | pane 배치 저니, KRX 인증 수집 대기 |
| `archive/USER_STORIES.md` | pane 배치·프리셋 저장 스토리 |
| `archive/SCREEN_ACTIONS.md` | PLACE/SWAP 행위 문법, pane 카탈로그 15종 |
| `archive/SCREEN_COMPOSITION.md` | 크롬·캔버스·pane 3층 구성 모델 |
