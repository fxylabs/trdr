#!/usr/bin/env python3
"""kit.css를 인라인해 kit.html 컴포넌트 갤러리를 조립한다."""
import pathlib

ROOT = pathlib.Path(__file__).resolve().parent.parent
KIT_CSS = (ROOT / "ui-kit" / "kit.css").read_text(encoding="utf-8")

GALLERY_BODY = """
<header class="g-head">
    <div class="k-logo" style="font-size:18px">trdr ui-kit</div>
    <div class="g-sub">디자인 원칙 10 기반 토큰·컴포넌트 갤러리 — 정본은 kit.css</div>
</header>

<section class="g-sec">
    <h2>토큰 — 배경 3층 · 시스템색 · 시장색</h2>
    <div class="g-row">
        <div class="sw" style="background:var(--k-bg-0)">bg-0<br><span>#121216 바탕·터미널</span></div>
        <div class="sw" style="background:var(--k-bg-1)">bg-1<br><span>#18181e pane 표면</span></div>
        <div class="sw" style="background:var(--k-bg-2)">bg-2<br><span>#1f1f27 타이틀·입력바</span></div>
        <div class="sw" style="background:var(--k-amber);color:#121216">amber<br><span>#ffb000 시스템색</span></div>
        <div class="sw" style="background:var(--k-up)">up<br><span>#f0524f 상승(적)</span></div>
        <div class="sw" style="background:var(--k-down)">down<br><span>#5b8aff 하락(청)</span></div>
    </div>
    <p class="g-note">순흑·glow·radius·그림자 없음. 시장색은 설정에서 적/청 반전 토글 전제.</p>
</section>

<section class="g-sec">
    <h2>상단 바</h2>
    <div class="k-app" style="height:auto">
        <div class="k-topbar">
            <span class="k-logo">trdr</span>
            <nav class="k-topbar-tabs">
                <button class="k-toptab on">콕핏</button>
                <button class="k-toptab">리서치</button>
                <button class="k-toptab">장부</button>
            </nav>
            <div class="k-topbar-right">
                <span class="k-badge k-badge--live">● KIS WS</span>
                <span class="k-badge">금융위 D+1</span>
                <span class="k-clock">14:32:05 KST</span>
            </div>
        </div>
    </div>
</section>

<section class="g-sec">
    <h2>pane + 고밀도 표 — tabular-nums 우측정렬 · ▲▼ 병기</h2>
    <div class="k-app" style="height:auto">
        <div class="k-pane" style="width:340px">
            <div class="k-pane-title">관심종목 <span class="k-badge k-badge--live">● LIVE</span></div>
            <table class="k-table">
                <tr><th>종목</th><th class="k-num">현재가</th><th class="k-num">등락</th></tr>
                <tr class="sel"><td>삼성전자 <span class="k-dim">005930</span></td>
                    <td class="k-num">87,300</td><td class="k-num k-up">▲ +1.04%</td></tr>
                <tr><td>SK하이닉스 <span class="k-dim">000660</span></td>
                    <td class="k-num">312,500</td><td class="k-num k-down">▼ -1.83%</td></tr>
                <tr><td>셀트리온 <span class="k-dim">068270</span></td>
                    <td class="k-num">194,700</td><td class="k-num k-flat">— 0.00%</td></tr>
            </table>
        </div>
    </div>
</section>

<section class="g-sec">
    <h2>배지 — provenance·상태</h2>
    <div class="k-app g-pad" style="height:auto">
        <span class="k-badge">금융위 API · D+1</span>
        <span class="k-badge k-badge--live">● KIS WS · LIVE</span>
        <span class="k-badge k-badge--amber">사전등록 2026-08-04</span>
        <span class="k-badge">replay 9f3a→77e0 ✓</span>
    </div>
</section>

<section class="g-sec">
    <h2>버튼</h2>
    <div class="k-app g-pad" style="height:auto">
        <button class="k-btn">취소</button>
        <button class="k-btn k-btn--ghost">스펙 열기</button>
        <button class="k-btn k-btn--amber">사전등록 — 해시 고정</button>
    </div>
</section>

<section class="g-sec">
    <h2>명령줄 (1급) + 팔레트 — 같은 자리가 에이전트 입력</h2>
    <div class="k-app" style="height:auto">
        <div class="k-palette">
            <div class="k-palette-row" style="border-bottom:1px solid var(--k-line-strong);cursor:default">
                <span class="k-mode-chip on">CMD</span>
                <span class="k-mode-chip">AGENT</span>
                <span class="k-palette-sub">Tab — 같은 입력창이 에이전트 프롬프트로 전환</span>
            </div>
            <div class="k-palette-row sel">
                <span class="k-palette-mn">BT</span>
                <span class="k-palette-desc">백테스트 실행</span>
                <span class="k-palette-sub">선택 전략: momentum-3</span>
            </div>
            <div class="k-palette-row">
                <span class="k-palette-mn">PREREG</span>
                <span class="k-palette-desc">사전등록 — 스펙 해시 고정</span>
            </div>
        </div>
        <div class="k-cmd">
            <span class="k-cmd-prompt">❯</span>
            <span class="k-cmd-token">005930</span>
            <span>BT</span><span class="k-cmd-caret"></span>
            <span class="k-cmd-hint">⌘K 명령 · Tab 에이전트</span>
        </div>
    </div>
</section>

<section class="g-sec">
    <h2>터미널 pane — BYO 에이전트 (xterm)</h2>
    <div class="k-app" style="height:auto">
        <div class="k-pane">
            <div class="k-pane-title">터미널 — BYO 에이전트 <span class="k-badge">xterm · node-pty</span></div>
            <div class="k-term" style="height:96px">
                <div><span class="t-prompt">❯</span> claude</div>
                <div class="t-out">✳ momentum-3 진입 조건을 스펙으로 형식화 → strategies/momentum-3.trdr.yaml</div>
                <div class="t-dim">⏺ 파일워처: 차트 오버레이 갱신됨 (momentum-3 @ 005930)</div>
            </div>
        </div>
    </div>
</section>

<section class="g-sec">
    <h2>화면 리드 — 화면당 리드 pane 하나, 답을 가장 큰 활자로</h2>
    <div class="k-app" style="height:auto;display:flex;gap:14px;padding:14px">
        <div class="k-pane k-pane--lead" style="flex:1">
            <div class="k-pane-title">005930 삼성전자 · 일봉 <span class="k-badge k-badge--live">● 실시간</span></div>
            <div class="k-ans">
                <span class="k-ans-fig">87,300</span>
                <span class="k-ans-delta k-up">▲ +1.04%</span>
                <span class="k-ans-sub">거래량 12,847천</span>
            </div>
            <div style="height:40px"></div>
        </div>
        <div class="k-pane k-pane--lead" style="flex:1">
            <div class="k-pane-title">내 전략 성적</div>
            <div class="k-ans">
                <span class="k-stamp k-stamp--sm">노이즈</span>
                <span class="k-ans-word">gap-open v2 — 결과 도착</span>
            </div>
            <div style="height:40px"></div>
        </div>
    </div>
    <p class="g-note">원칙 6(스탬프 주인공)의 일반화 — 리드 pane은 타이틀에 앰버 틱,
    답 스트립(.k-ans)에 화면의 답 하나. 목록 pane은 역할 타이틀("오늘 볼 종목")이
    답 스트립을 대신한다. 질문 정의는 SCREEN_ACTIONS §2.</p>
</section>

<section class="g-sec">
    <h2>판정 스탬프 — 시각 주인공 · 축하 없음</h2>
    <div class="k-app g-pad" style="height:auto;display:flex;gap:40px;align-items:center;flex-wrap:wrap">
        <div style="text-align:center">
            <div class="k-stamp">노이즈<small>NOISE · NOT SIGNIFICANT</small></div>
            <div class="k-stamp-meta" style="margin-top:14px">p=0.41 · oos sharpe 0.12 · 2026-08-06 판정</div>
        </div>
        <div style="text-align:center">
            <div class="k-stamp k-stamp--sm">사전등록<small>REGISTERED</small></div>
            <div class="k-stamp-meta" style="margin-top:10px">sha256:9f3a…c21b · 2026-08-04 09:12 KST</div>
        </div>
    </div>
</section>

<section class="g-sec">
    <h2>OOS 카운트다운 + 고지</h2>
    <div class="k-app g-pad" style="height:auto;display:flex;gap:28px;align-items:center;flex-wrap:wrap">
        <div>
            <div class="k-countdown">D-23</div>
            <div class="k-countdown-label">OOS 판정까지 잔여 거래일</div>
        </div>
        <div class="k-notice" style="max-width:340px">
            스펙 잠김 — 등록 후 수정은 새 등록을 만들고, 재등록 이력은 판정 리포트에 표시됩니다.
        </div>
    </div>
</section>
"""

PAGE = f"""<!DOCTYPE html>
<html lang="ko">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>trdr ui-kit — 컴포넌트 갤러리</title>
<style>
{KIT_CSS}

/* ── 갤러리 전용 (mk- 상당) ── */
* {{ margin: 0; padding: 0; box-sizing: border-box; }}
body {{
    background: #0d0d10;
    color: var(--k-ink);
    font-family: var(--k-sans);
    padding: 32px;
    max-width: 980px;
    margin: 0 auto;
}}
.g-head {{ margin-bottom: 28px; }}
.g-sub {{ color: var(--k-ink-2); font-size: 12.5px; margin-top: 6px; }}
.g-sec {{ margin-bottom: 34px; }}
.g-sec h2 {{
    font-size: 12px; font-weight: 700; letter-spacing: .08em;
    color: var(--k-ink-2); margin-bottom: 10px;
    border-bottom: 1px solid var(--k-line); padding-bottom: 6px;
}}
.g-row {{ display: flex; gap: 8px; flex-wrap: wrap; }}
.g-note {{ color: var(--k-ink-3); font-size: 11px; margin-top: 8px; }}
.g-pad {{ padding: 16px; }}
.sw {{
    width: 130px; height: 64px; padding: 8px;
    font: 700 11px var(--k-mono); color: var(--k-ink);
    border: 1px solid var(--k-line);
}}
.sw span {{ font-weight: 500; color: inherit; opacity: .75; font-size: 10px; }}
</style>
</head>
<body>
{GALLERY_BODY}
</body>
</html>
"""

out = ROOT / "ui-kit" / "kit.html"
out.write_text(PAGE, encoding="utf-8")
print(f"wrote {out} ({len(PAGE):,} bytes)")
