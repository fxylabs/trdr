#!/usr/bin/env python3
"""trdr 신규 표면 목업 조립기 (목업 v2 — 파일 2).

USER_STORIES.md의 v2 신규(●) 표면: 온보딩 / 내 가설의 오늘 / 리서치 비교 /
검증 기록 / 수집 상태·설정. 공용 셸·SVG는 assemble_flow에서 가져온다.
"""
import pathlib
import re

from assemble_flow import (KIT_CSS, MK_CSS, TEMPLATE, app, cmdline, line_svg,
                           walk)

ROOT = pathlib.Path(__file__).resolve().parent.parent
OUT = ROOT / "2026-08-06_trdr-surfaces-flow.html"
TITLE = "trdr 화면 목업 — 온보딩·일상·기록·상태"
VERSION = "v2"


# ─────────────────────────────────────────────────────────────────
# 1. 온보딩 (US-01·02)
# ─────────────────────────────────────────────────────────────────

def screen_onboarding():
    inner = """
    <div class="mk-report">
        <div class="k-pane" style="width:720px">
            <div class="k-pane-title">처음 설정 <span class="k-badge">2/3 단계</span></div>
            <div style="padding:20px 24px;display:flex;flex-direction:column;gap:16px">
                <div class="mk-step done">
                    <span class="mk-stepno">1</span>
                    <div style="flex:1">
                        <b>API 키 등록</b> — 완료
                        <div class="k-dim" style="font-size:11px;margin-top:2px">
                            KIS ✓ · DART ✓ · 금융위 ✓ — 모두 macOS Keychain에 저장됨.
                            키 값은 화면·로그 어디에도 표시되지 않습니다.</div>
                    </div>
                </div>
                <div class="mk-step now">
                    <span class="mk-stepno">2</span>
                    <div style="flex:1">
                        <b>데이터셋 받기</b> — 진행 중 <span class="k-num k-dim" style="font-size:11px">trdr 수집 --전체</span>
                        <div class="mk-prog-row"><span>일봉 (KIS)</span><div class="mk-prog"><i style="width:100%"></i></div><span class="k-num">1,173종목 ✓</span></div>
                        <div class="mk-prog-row"><span>수급 (KRX)</span><div class="mk-prog"><i style="width:62%"></i></div><span class="k-num">62%</span></div>
                        <div class="mk-prog-row"><span>공시 (DART)</span><div class="mk-prog"><i style="width:8%"></i></div><span class="k-num">대기</span></div>
                        <div class="k-notice" style="margin-top:8px" data-toast="US-02 — 실패 지점·원인·재개 명령을 함께 보여준다. 부분 완료분은 보존">
                            ⚠ 수급 수집이 08-05 지점에서 멈췄습니다 — KRX 로그인 만료.
                            받은 데이터는 보존됩니다. 재개: <b class="k-num" style="font-size:11px">trdr 수집 --재개</b></div>
                    </div>
                </div>
                <div class="mk-step">
                    <span class="mk-stepno">3</span>
                    <div style="flex:1"><b>백테스트 가능</b>
                        <div class="k-dim" style="font-size:11px;margin-top:2px">완료되면 첫 가설을 에이전트와 만들 수 있습니다.</div>
                    </div>
                </div>
                <div style="display:flex;gap:8px;justify-content:flex-end">
                    <button class="k-btn" data-toast="수집은 뒤에서 계속 — 앱을 먼저 둘러볼 수 있다">앱 먼저 둘러보기</button>
                    <button class="k-btn k-btn--amber" data-goto="today">계속</button>
                </div>
            </div>
        </div>
    </div>"""
    cmd = cmdline('<span class="k-dim">수집이 끝나면 알려드립니다</span>', hint="")
    return f"""
    <section class="flow-step" data-screen="onboarding" data-title="온보딩">
        <div class="step-head">첫 실행 온보딩 <small>US-01·02</small></div>
        <div class="device mk-device">{app(inner, cmd)}</div>
        <div class="step-note">키 등록(keychain 전용) → 데이터셋 명령 1개 → 백테스트 가능까지 3단계. 실패는 숨기지 않고 원인+재개 명령을 그 자리에서 보여준다.</div>
    </section>"""


# ─────────────────────────────────────────────────────────────────
# 2. 내 가설의 오늘 (US-03·04)
# ─────────────────────────────────────────────────────────────────

def screen_today():
    shadow = walk(12, seed=21, drift=0.21, vol=0.55)
    bench = walk(12, seed=22, drift=0.16, vol=0.4)
    svg = line_svg(
        [("전략 shadow", "var(--k-amber)", "", shadow), ("KOSPI200", "var(--k-ink-3)", "4 3", bench)],
        w=560, h=130,
    )
    inner = f"""
    <div class="mk-main" style="padding:8px;gap:8px">
        <div style="flex:1;display:flex;flex-direction:column;gap:8px;min-width:0">
            <div class="k-pane">
                <div class="k-pane-title">결과 도착 <span class="k-badge k-badge--amber">1건</span></div>
                <div class="mk-card" data-goto="registry">
                    <div class="k-stamp k-stamp--sm" style="font-size:13px">노이즈<small>NOISE</small></div>
                    <div style="flex:1">
                        <b>gap-open v2</b> — 결과: 노이즈 <span class="k-dim">(07-30 · 실전 관찰)</span>
                        <div class="k-dim" style="font-size:11px">p=0.38 · 검증 35 거래일 · 리포트 보기 →</div>
                    </div>
                </div>
            </div>
            <div class="k-pane">
                <div class="k-pane-title">관찰 중 <span class="k-badge">1건</span></div>
                <div style="display:flex;gap:8px;align-items:flex-start">
                    <div class="mk-card" style="flex:1" data-toast="관찰 pane — D-0까지 참고용 지표만">
                        <div class="k-countdown" style="font-size:30px">D-23</div>
                        <div style="flex:1">
                            <b>momentum-3 v4</b> <span class="k-dim">실전 관찰 · 관측 12/35</span>
                            <div class="k-dim" style="font-size:11px">참고: +2.31% vs KOSPI200 +1.87%</div>
                        </div>
                    </div>
                    <div style="width:340px;padding:8px 10px 4px">{svg}</div>
                </div>
            </div>
            <div class="k-pane" style="flex:1">
                <div class="k-pane-title">오늘 공시 <span class="k-badge">DART</span></div>
                <div class="k-pane-body" style="overflow:auto">
                <table class="k-table">
                    <tr><td>삼성전자</td><td>반기보고서 제출</td><td class="k-num k-dim">14:02</td></tr>
                    <tr><td>카카오</td><td>주요사항보고 (자기주식)</td><td class="k-num k-dim">11:40</td></tr>
                    <tr><td>한화오션</td><td>단일판매·공급계약</td><td class="k-num k-dim">09:15</td></tr>
                </table>
                </div>
            </div>
        </div>
        <div class="k-pane" style="width:300px;flex:none">
            <div class="k-pane-title">관심종목 <span class="k-badge" data-toast="US-04 — 실시간 슬롯 표시. 초과분은 지연 라벨">실시간 41/41</span></div>
            <div class="k-pane-body" style="overflow:auto">
            <table class="k-table">
                <tr><th>종목</th><th class="k-num">현재가</th><th class="k-num">등락</th></tr>
                <tr><td>삼성전자</td><td class="k-num">87,300</td><td class="k-num k-up">▲ +1.04%</td></tr>
                <tr><td>SK하이닉스</td><td class="k-num">312,500</td><td class="k-num k-down">▼ -1.83%</td></tr>
                <tr><td>NAVER</td><td class="k-num">231,500</td><td class="k-num k-up">▲ +2.21%</td></tr>
                <tr><td>카카오</td><td class="k-num">48,150</td><td class="k-num k-down">▼ -0.72%</td></tr>
                <tr><td>두산에너빌리티<br><span class="k-badge" data-toast="실시간 슬롯 초과 — 15분 지연 시세로 표시 중">지연 15분</span></td><td class="k-num">33,700</td><td class="k-num k-up">▲ +2.74%</td></tr>
                <tr><td>한화오션<br><span class="k-badge">지연 15분</span></td><td class="k-num">41,200</td><td class="k-num k-flat">— 0.00%</td></tr>
            </table>
            </div>
        </div>
    </div>"""
    cmd = cmdline('<span class="k-dim">005930 — 명령 또는 에이전트 입력…</span>', hint="⌘K 명령 · Tab 에이전트")
    return f"""
    <section class="flow-step" data-screen="today" data-title="내 가설의 오늘">
        <div class="step-head">프리셋 — 내 가설의 오늘 <small>US-03·04 · daily-open</small></div>
        <div class="device mk-device">{app(inner, cmd, active="오늘")}</div>
        <div class="step-note">아침에 여는 화면. 결과 도착이 최상단, 관찰 카운트다운, 그리고 내 배치의 시세·공시. 등록 0건이면 이 자리에 "첫 가설을 만들어보세요 — Tab" 빈 상태가 뜬다.</div>
    </section>"""


# ─────────────────────────────────────────────────────────────────
# 3. 리서치 비교 (US-08)
# ─────────────────────────────────────────────────────────────────

def screen_research():
    lines = [
        ("v4", "var(--k-amber)", "", walk(40, seed=41, drift=0.5, vol=0.8)),
        ("v3", "var(--k-ink-2)", "", walk(40, seed=42, drift=0.34, vol=0.9)),
        ("v2", "var(--k-ink-3)", "4 3", walk(40, seed=43, drift=0.1, vol=0.9)),
    ]
    svg = line_svg(lines, w=820, h=170)
    inner = f"""
    <div class="mk-report">
        <div class="k-pane" style="width:900px">
            <div class="k-pane-title">백테스트 비교 — momentum-3
                <span style="margin-left:auto;display:flex;gap:6px">
                    <span class="k-badge k-badge--amber" data-toast="탐색 결과와 검증 결과를 시각적으로 구분 — 탐색은 증거가 아니다">탐색 결과 — 검증 아님</span>
                    <span class="k-badge">구간 2019-01-02 ~ 2025-12-30 고정</span>
                </span>
            </div>
            <div style="padding:18px 22px;display:flex;flex-direction:column;gap:14px">
                <div style="display:flex;gap:14px;font-size:10.5px">
                    <span class="k-amber-t">— v4 (top 3)</span><span class="k-sec">— v3 (top 5)</span><span class="k-dim">┄ v2 (top 10)</span>
                </div>
                {svg}
                <table class="k-table">
                    <tr><th>버전</th><th>차이</th><th class="k-num">sharpe</th><th class="k-num">수익률(연)</th><th class="k-num">최대낙폭</th><th class="k-num">거래</th><th></th></tr>
                    <tr class="sel"><td>v4</td><td>top 3 · 월간</td><td class="k-num">1.31</td><td class="k-num k-up">▲ +18.4%</td><td class="k-num k-down">▼ -9.7%</td><td class="k-num">274회</td><td><button class="k-btn" style="padding:2px 8px;font-size:10.5px" data-toast="등록 화면으로 — 검증 모드 선택 후 사람이 확정">등록</button></td></tr>
                    <tr><td>v3</td><td>top 5 · 월간</td><td class="k-num">0.94</td><td class="k-num k-up">▲ +13.1%</td><td class="k-num k-down">▼ -11.2%</td><td class="k-num">452회</td><td><span class="k-badge">결과: 노이즈 (과거 검증)</span></td></tr>
                    <tr><td>v2</td><td>top 10 · 주간</td><td class="k-num">0.41</td><td class="k-num k-up">▲ +6.0%</td><td class="k-num k-down">▼ -14.8%</td><td class="k-num">1,203회</td><td></td></tr>
                </table>
                <div class="k-notice">같은 탐색 구간·같은 비용 가정으로만 비교합니다. 여기 숫자는 <b>탐색 결과</b>입니다 — 검증을 거쳐야 결과가 됩니다.</div>
            </div>
        </div>
    </div>"""
    cmd = cmdline('<span class="k-cmd-token">momentum-3</span> <span>백테스트 --비교</span>', hint="한/영 모두 인식")
    return f"""
    <section class="flow-step" data-screen="research" data-title="리서치 비교">
        <div class="step-head">리서치 — 백테스트 비교 <small>US-08</small></div>
        <div class="device mk-device">{app(inner, cmd)}</div>
        <div class="step-note">버전 간 비교는 축(구간·비용)이 고정된 상태에서만. "탐색 결과 — 검증 아님" 라벨이 상시 노출 — 탐색 성적을 증거처럼 읽는 자기 기만을 막는다. v3의 과거 검증 결과가 계보로 보인다.</div>
    </section>"""


# ─────────────────────────────────────────────────────────────────
# 4. 검증 기록 (US-16·13)
# ─────────────────────────────────────────────────────────────────

def screen_registry():
    inner = """
    <div class="mk-report">
        <div class="k-pane" style="width:900px">
            <div class="k-pane-title">검증 기록
                <span style="margin-left:auto;display:flex;gap:6px">
                    <span class="k-badge" data-toast="삭제·편집 UI 자체가 없다 — 기록은 남는다">불변 기록</span>
                </span>
            </div>
            <div style="padding:14px 18px;display:flex;flex-direction:column;gap:12px">
                <div style="display:flex;gap:6px">
                    <span class="k-mode-chip on">전체 5</span>
                    <span class="k-mode-chip">관찰 중 1</span>
                    <span class="k-mode-chip">노이즈 3</span>
                    <span class="k-mode-chip">유효 0</span>
                    <span class="k-mode-chip">과거 검증 2</span>
                    <span class="k-mode-chip">실전 관찰 3</span>
                </div>
                <table class="k-table">
                    <tr><th>등록</th><th>전략</th><th>모드</th><th class="k-num">등록일</th><th>상태 / 결과</th><th>계보</th></tr>
                    <tr class="sel" data-toast="관찰 pane으로 이동">
                        <td class="k-num" style="text-align:left">r-0005</td><td><b>momentum-3 v4</b></td><td>실전 관찰</td>
                        <td class="k-num">08-04</td><td><span class="k-amber-t">관찰 중 · D-23</span></td><td class="k-dim">← v3</td></tr>
                    <tr data-toast="결과 리포트 열람">
                        <td class="k-num" style="text-align:left">r-0004</td><td>momentum-3 v3</td><td>과거 검증</td>
                        <td class="k-num">08-01</td><td>결과: 노이즈 <span class="k-dim">(p=0.22)</span></td><td class="k-dim">← v2 · → v4</td></tr>
                    <tr data-toast="결과 리포트 열람">
                        <td class="k-num" style="text-align:left">r-0003</td><td>gap-open v2</td><td>실전 관찰</td>
                        <td class="k-num">06-12</td><td>결과: 노이즈 <span class="k-dim">(07-30 · p=0.38)</span></td><td></td></tr>
                    <tr data-toast="결과 리포트 열람">
                        <td class="k-num" style="text-align:left">r-0002</td><td>momentum-3 v2</td><td>과거 검증</td>
                        <td class="k-num">07-28</td><td>결과: 노이즈 <span class="k-dim">(p=0.51)</span></td><td class="k-dim">→ v3</td></tr>
                    <tr data-toast="첫 등록 — 스펙 오류로 무효 처리된 기록도 남는다">
                        <td class="k-num" style="text-align:left">r-0001</td><td>gap-open v1</td><td>실전 관찰</td>
                        <td class="k-num">05-30</td><td class="k-dim">무효 (스펙 오류)</td><td class="k-dim">→ v2</td></tr>
                </table>
                <div class="k-notice">기록은 지우거나 고칠 수 없습니다. 노이즈 3건도 이 전략 개발의 정직한 이력입니다 — 계보(←·→)가 무엇을 고쳐 다시 시도했는지 보여줍니다.</div>
            </div>
        </div>
    </div>"""
    cmd = cmdline('<span class="k-cmd-token"></span><span>기록</span>', hint="hist")
    return f"""
    <section class="flow-step" data-screen="registry" data-title="검증 기록">
        <div class="step-head">검증 기록 <small>US-16·13 · 불변</small></div>
        <div class="device mk-device">{app(inner, cmd, active="검증 기록")}</div>
        <div class="step-note">시간순 등록 이력 + 필터 + 계보. 삭제·편집 버튼이 존재하지 않는 것 자체가 요구사항. 무효 처리된 첫 등록도 남는다.</div>
    </section>"""


# ─────────────────────────────────────────────────────────────────
# 5. 수집 상태 + 설정 (US-17·18)
# ─────────────────────────────────────────────────────────────────

def screen_datastatus():
    inner = """
    <div class="mk-report" style="gap:10px;display:flex;align-items:flex-start;justify-content:center">
        <div class="k-pane" style="width:470px">
            <div class="k-pane-title">수집 상태 <span class="k-badge">trdr 수집 --상태</span></div>
            <div style="padding:12px 14px;display:flex;flex-direction:column;gap:10px">
                <table class="k-table">
                    <tr><th>소스</th><th class="k-num">최신</th><th class="k-num">규모</th><th>상태</th></tr>
                    <tr><td>일봉 (KIS)</td><td class="k-num">08-06</td><td class="k-num">1,173종목</td><td>✓ 정상</td></tr>
                    <tr><td>수급 (KRX)</td><td class="k-num">08-05</td><td class="k-num">1,146종목</td><td class="k-amber-t" data-toast="실패 원인과 재개 명령을 행에서 바로 — US-17">⚠ 로그인 만료</td></tr>
                    <tr><td>공시 (DART)</td><td class="k-num">08-06</td><td class="k-num">7,263건</td><td>✓ 정상</td></tr>
                    <tr><td>기준 (금융위)</td><td class="k-num">08-06</td><td class="k-num">2,041종목</td><td>✓ 정상</td></tr>
                    <tr><td>상폐 명부</td><td class="k-num">08-06</td><td class="k-num">4,052건</td><td>✓ 정상</td></tr>
                </table>
                <div class="k-notice">수급: KRX 로그인이 만료됐습니다. keychain의 자격증명으로 재로그인 후 재개하세요 — <b class="k-num" style="font-size:11px">trdr 수집 --재개</b></div>
                <div style="display:flex;gap:8px">
                    <button class="k-btn k-btn--ghost" data-toast="전체 소스 수집 트리거 — 에이전트도 같은 CLI로 실행 가능">지금 수집</button>
                    <button class="k-btn" data-toast="출처 라벨 상세 — 소스·기간·보정 이력">출처 상세</button>
                </div>
            </div>
        </div>
        <div class="k-pane" style="width:400px">
            <div class="k-pane-title">설정</div>
            <div style="padding:12px 14px;display:flex;flex-direction:column;gap:12px">
                <div>
                    <div style="font-size:11px;font-weight:700;color:var(--k-ink-2);margin-bottom:6px">API 키 (Keychain)</div>
                    <div class="k-kv"><dt>kis-app-key</dt><dd data-toast="US-18 — 존재 여부만. 값은 어떤 화면에도 없다">등록됨 · 값 표시 불가</dd></div>
                    <div class="k-kv"><dt>dart-api-key</dt><dd>등록됨 · 값 표시 불가</dd></div>
                    <div class="k-kv"><dt>fsc-api-key</dt><dd>등록됨 · 값 표시 불가</dd></div>
                    <div class="k-kv"><dt>krx-id / krx-pw</dt><dd class="k-amber-t">재로그인 필요</dd></div>
                </div>
                <hr class="k-hr" style="margin:0">
                <div>
                    <div style="font-size:11px;font-weight:700;color:var(--k-ink-2);margin-bottom:6px">시장색</div>
                    <div style="display:flex;gap:6px">
                        <span class="k-mode-chip on" data-toast="한국 관습 기본값 — 시장 정보만 바뀐다. 앰버·결과 표시는 불변">적상 / 청하</span>
                        <span class="k-mode-chip">반전 (green up)</span>
                    </div>
                </div>
                <hr class="k-hr" style="margin:0">
                <div>
                    <div style="font-size:11px;font-weight:700;color:var(--k-ink-2);margin-bottom:6px">레이아웃 프리셋</div>
                    <div class="k-kv"><dt>내 가설의 오늘</dt><dd class="k-dim">기본</dd></div>
                    <div class="k-kv"><dt>리서치</dt><dd class="k-dim">기본</dd></div>
                    <div class="k-kv"><dt>내 배치 1</dt><dd><button class="k-btn" style="padding:1px 8px;font-size:10px" data-toast="프리셋 저장·이름 변경·삭제 — 레이아웃은 사용자 소유">관리</button></dd></div>
                </div>
            </div>
        </div>
    </div>"""
    cmd = cmdline('<span class="k-cmd-token"></span><span>수집</span>', hint="data")
    return f"""
    <section class="flow-step" data-screen="datastatus" data-title="수집·설정">
        <div class="step-head">수집 상태 · 설정 <small>US-17·18</small></div>
        <div class="device mk-device">{app(inner, cmd)}</div>
        <div class="step-note">소스별 최신·규모·상태를 표 하나로. 실패 행에 원인과 재개 명령. 설정의 키는 존재 여부만 — 값은 어느 화면에도 없다 (keychain 전용 결정).</div>
    </section>"""


# ─────────────────────────────────────────────────────────────────
# 조립
# ─────────────────────────────────────────────────────────────────

MK2_CSS = """
/* ── 표면 목업 전용 (mk-, 파일 2) ── */
.mk-step { display: flex; gap: 12px; align-items: flex-start; padding: 10px 12px; border: 1px solid var(--k-line); font-size: 12px; }
.mk-step.now { border-color: var(--k-amber-dim); background: var(--k-amber-bg); }
.mk-step.done { color: var(--k-ink-2); }
.mk-stepno { font: 700 12px var(--k-mono); color: var(--k-amber); border: 1px solid var(--k-amber-dim); width: 22px; height: 22px; display: inline-flex; align-items: center; justify-content: center; flex: none; }
.mk-prog-row { display: flex; align-items: center; gap: 10px; margin-top: 7px; font-size: 11px; }
.mk-prog-row > span:first-child { width: 84px; color: var(--k-ink-2); flex: none; }
.mk-prog { flex: 1; height: 8px; background: var(--k-bg-0); border: 1px solid var(--k-line); }
.mk-prog i { display: block; height: 100%; background: var(--k-amber); }
.mk-card { display: flex; gap: 14px; align-items: center; padding: 10px 12px; margin: 8px 10px; border: 1px solid var(--k-line); cursor: pointer; font-size: 12px; }
.mk-card:hover { border-color: var(--k-ink-3); }
"""

PRESET_NOTES = """
    { screen: "onboarding", x: 50, y: 30, text: "keychain 전용 결정의 화면화 — 키 값이 존재하는 자리가 UI에 아예 없다. 실패는 원인+재개 명령과 함께 그 자리에서." },
    { screen: "today", x: 33, y: 14, text: "daily-open의 근거: '내가 걸어둔 가설이 오늘 어떻게 됐나'. 결과 도착이 최상단 — 노이즈여도 같은 자리." },
    { screen: "today", x: 82, y: 40, text: "US-04 — 실시간 슬롯 41 상한을 숨기지 않는다. 지연 종목은 라벨로 구분해 옛 가격을 지금 가격으로 착각하지 않게." },
    { screen: "research", x: 50, y: 12, text: "탐색 결과 라벨 상시 노출 — 탐색 성적은 증거가 아니다. 검증(등록) 버튼이 각 행에서 한 클릭." },
    { screen: "registry", x: 50, y: 40, text: "삭제·편집 UI의 부재가 곧 요구사항(US-16). 노이즈 이력과 계보가 '정직한 개발 이력'으로 읽히는 게 목표." },
    { screen: "datastatus", x: 72, y: 30, text: "키는 존재 여부만(US-18). 값을 보여주는 화면이 설계상 존재하지 않는다." }
"""


def main():
    html = TEMPLATE.read_text(encoding="utf-8")
    html = html.replace("<title>주제 이름 — 플로우 목업</title>", f"<title>{TITLE}</title>")
    html = html.replace('<div class="deck-title">주제 이름', f'<div class="deck-title">{TITLE.split(" — ")[0]}')

    kit_block = ("/* ═══ [KIT] trdr ui-kit 인라인 (정본: artifacts/ui-kit/kit.css) ═══ */\n"
                 + KIT_CSS + MK_CSS + MK2_CSS + "\n/* ═══ /[KIT] ═══ */")
    html = re.sub(r"/\* ═══ \[KIT\].*?/\[KIT\] ═══ \*/", lambda m: kit_block, html, flags=re.S)

    screens = "\n".join([
        screen_onboarding(),
        '<div class="flow-arrow" data-label="설정 완료 — 첫 화면"></div>',
        screen_today(),
        '<div class="flow-arrow" data-label="가설 작업 (프리셋 전환)"></div>',
        screen_research(),
        '<div class="flow-arrow" data-label="등록·결과의 이력"></div>',
        screen_registry(),
        '<div class="flow-arrow" data-label="상태 점검"></div>',
        screen_datastatus(),
    ])
    screens_block = "<!-- ═══ [SCREENS] ═══ -->\n" + screens + "\n<!-- ═══ /[SCREENS] ═══ -->"
    html = re.sub(r"<!-- ═══ \[SCREENS\].*?/\[SCREENS\] ═══ -->", lambda m: screens_block, html, flags=re.S)

    html = re.sub(r"var PRESET_NOTES = \[.*?\];",
                  lambda m: "var PRESET_NOTES = [" + PRESET_NOTES + "];", html, flags=re.S)
    html = html.replace('var MOCKUP_VERSION = "v1";', f'var MOCKUP_VERSION = "{VERSION}";')
    html = html.replace(
        "<li><b>v1</b> 최초 작성</li>",
        "<li><b>v2</b> 최초 작성 (목업 v2 신규 표면 — USER_STORIES.md ● 11건: 온보딩·내 가설의 오늘·리서치 비교·검증 기록·수집/설정)</li>",
    )

    OUT.write_text(html, encoding="utf-8")
    print(f"wrote {OUT} ({len(html):,} bytes)")


if __name__ == "__main__":
    main()
