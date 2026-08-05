> Origin: relationship-explorer w-v3aq0 report 01kz9bhw2928ynrrhjh67jae5r (2026-08-06)

# trdr MVP 범위 계약 (v0, review_ready 초안 — 2026-08-06)

근거 결정: 제품화 v4(01kz99bazcw85e9ms0pbpvfesp) · 출시 순서(01kz99m8qywddj8cxwgdrx8452) · 데이터 경계(01kz97yrasgf9bjfkf70xmsk5m 계보) · 이름/식별자(01kz9b3gekfqf5m6n84342dse3 계보) · 조사 3건(w-a09bb 규제·데이터, w-v3aq0 기술·디자인 artifact).

## v0의 완료 정의
1단계(로컬 디벨롭)의 끝 = "공개 가능한 수준" + "후킹 영상 재료가 나오는 상태". 공개 자체는 아님.
dogfooding 인수 기준: 이 연구 프로젝트의 H1급 워크플로(수집→스펙 확정→백테스트→판정→shadow 관찰)를 trdr 안에서 수행할 수 있다.

## v0 포함 (빌드 순서대로)
1. **코어 엔진**: 전략 스펙 포맷(선언적, 파일 기반) + 스펙 해시·타임스탬프 사전등록 장부(로컬) + deterministic replay 엔진(기존 T0 자산 이식) + out-of-sample 판정 리포트(노이즈 판정 포함).
2. **데이터 층**: canonical 스키마 + importer 계약 공개 스펙. 수집기 3종: 금융위 API(사전 제공 베이스), DART, KIS 로컬(본인 키·기기 한정). KDM CSV importer 1급. 모든 판정에 provenance 라벨.
3. **콕핏 셸**: Electron + dockview + lightweight-charts v5(attribution 고지) + xterm.js 터미널 pane(BYO 에이전트). KIS 웹소켓 실시간 차트(본인 키, 세션당 ~41종목). 명령줄 1급(티커+니모닉 = 에이전트 입력 자리). 디자인 원칙 10 준수.
4. **에이전트 통합**: 앱 MCP 서버(백테스트 실행·차트 조작·데이터 조회 도구) + 파일워처(전략 파일 저장 → 차트 오버레이·판정 핫리로드). 에이전트 비종속(Claude Code·Codex 동일 동작).
5. **판정 스탬프·리포트 폴리시**: 사전등록 도장, oos 대기 카운트다운, 축하 없음. 1080p 축소 검수 통과.

영상 재료는 3 이후 단계마다 뽑는다(코어만으로는 그림이 안 나옴).

## v0 명시적 보류
- 모바일 앱, hosted agent API, 클라우드(백업·공증 서버·러너), 커뮤니티 플러그인.
- 브라우저 로그인 pane — 토스 공식 API 등장으로 우선순위 하락, v1 후보로 강등.
- 템플릿 전략의 알림 연결(규제선), 전략 공유·마켓, 미국 데이터.
- 도메인·npm 배포·상표 — 공개 시점 일괄.

## 품질·검증 게이트
- 금융위 API 실호출 검증(이력 깊이·상폐 포함·거래대금)이 수집기 구현의 첫 블로킹 게이트.
- replay 결정론: 동일 input hash → 동일 output hash 테스트를 v0부터 유지(기존 T0 방식).
- 디자인: 원칙 10 체크리스트 + 1080p 검수를 PR 게이트로.
- 브로커 약관 서면확인·비조치의견서·변호사 검토는 각각 모바일 매매·클라우드 러너·공유 기능 전 게이트 — v0 무관.

## 미결(사용자 결정 필요)
1. **구현 모델 배정**: relationship-explorer의 Luna 전용 컨벤션이 trdr repo에도 적용되는가, 아니면 별도 배정인가.
2. **OSS 라이선스**: MIT(채택 극대화) vs AGPL(OpenBB가 클라우드 방어용으로 선택) — 공개 전까지 결정하면 되나 코드 헤더·의존성 정리에 영향.
3. trdr를 superself 별도 프로젝트로 등록할지(현재는 relationship-explorer work 그래프에 얹혀 있음).
