# 보관 문서

여기 있는 문서는 2026-08-06~08-07에 쓰였고, 2026-08-10에 확정된 계약과 어긋난다. 구현 입력이
아니지만 지우지 않는다. 지금의 계약이 어떤 판단을 거쳐 나왔는지 이 문서들이 증거로 남는다.

각 파일은 본문이 원문 그대로이고, 머리에 보관 사유와 대체 문서만 덧붙였다.

## 무엇이 이 문서들을 대체했나

2026-08-10에 확정된 결정 4가지가 여섯 문서를 한꺼번에 무효화했다.

| 폐기된 전제 | 확정된 계약 |
|---|---|
| Electron + dockview 셸, pane 자유 배치와 프리셋 저장 | Tauri 2 고정 단일 창. Today/Lab/Strategies와 오른쪽 raw terminal rail |
| KRX 수급 이력을 공식 수집기로 제공 | 공식 내장 collector는 KIS·OpenDART·ECOS. KRX는 공식 경로에서 제외하고, 사용자가 확보한 자료만 `user.*` bundle로 적재 |
| 앱 MCP 서버로 에이전트가 앱을 조작 | 에이전트는 파일과 `trdr` CLI만 쓴다. CLI는 Unix socket으로 실행 중 앱에 요청한다 |
| 사용자 1명(user zero) 인수 기준 | 외부 목표 사용자 5~10명의 activation·retention·유료 수용 기준 |

## 파일별

| 파일 | 원문 시점 | 대체 문서 |
|---|---|---|
| `PRD.md` | 2026-08-06 | `docs/IMPLEMENTATION_PLAN.md` 1~4장, `docs/FOUNDATION_DESIGN.md`, `artifacts/ui-kit/README.v2.md` |
| `MVP_SCOPE.md` | 2026-08-06 | `docs/IMPLEMENTATION_PLAN.md` 1~4장, `LICENSE`, `AGENTS.md`의 프로젝트 컨벤션 |
| `USER_JOURNEY.md` | 2026-08-06 | `docs/IMPLEMENTATION_PLAN.md` 4장 |
| `USER_STORIES.md` | 2026-08-06 | `docs/IMPLEMENTATION_PLAN.md` 2장, `artifacts/ui-kit/contracts.v2.json` |
| `SCREEN_ACTIONS.md` | 2026-08-07 | `artifacts/ui-kit/contracts.v2.json`, `docs/FOUNDATION_DESIGN.md` 9장 |
| `SCREEN_COMPOSITION.md` | 2026-08-07 | `artifacts/ui-kit/README.v2.md`, `contracts.v2.json` |

`MVP_SCOPE.md` 끝의 미결 3건은 모두 결정됐다. 구현 모델은 Opus 5로 배정했고(`AGENTS.md`),
라이선스는 AGPLv3로 확정했으며(`LICENSE`), trdr는 superself의 별도 프로젝트로 등록됐다.

## 살아남은 것

문서 전체가 폐기됐다는 뜻은 아니다. 아래 판단은 현재 계약에도 그대로 있다.

- 사전등록·검증·결과의 정직성 규율, 축하 연출 없는 결과 표시
- 절차의 무게는 시각으로, 용어는 일상어로
- 주문·실행은 v0에 존재하지 않는다
- 자격증명은 Keychain 전용, 값은 어디에도 노출하지 않는다
- 결과에는 출처 라벨과 재현 검증 해시가 함께 간다
