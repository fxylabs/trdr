#!/usr/bin/env python3
"""trdr 콕핏 플로우 목업 조립기.

skill template/flow.html + ui-kit/kit.css + 화면 조각(이 파일의 함수들)을
마커 치환으로 합쳐 artifacts/2026-08-06_trdr-cockpit-flow.html을 만든다.
피드백 반영 시: 이 파일의 조각을 고치고 다시 실행한다.
"""
import pathlib
import random
import re

ROOT = pathlib.Path(__file__).resolve().parent.parent
TEMPLATE = pathlib.Path("/Users/launchscreen/.claude/skills/mockup-loop/template/flow.html")
OUT = ROOT / "2026-08-06_trdr-cockpit-flow.html"
KIT_CSS = (ROOT / "ui-kit" / "kit.css").read_text(encoding="utf-8")

TITLE = "trdr 메인 플로우 — 목업"
VERSION = "v2"


# ─────────────────────────────────────────────────────────────────
# SVG 생성 (결정론 — seed 고정)
# ─────────────────────────────────────────────────────────────────

def fmt(n):
    return f"{int(round(n)):,}"


def candles_svg(w=740, h=464, n=72, seed=7):
    rng = random.Random(seed)
    price = 78000.0
    bars = []
    for i in range(n):
        o = price
        drift = 0.0016 if i > n * 0.5 else -0.0004
        c = o * (1 + rng.gauss(drift, 0.011))
        hi = max(o, c) * (1 + abs(rng.gauss(0, 0.0035)))
        lo = min(o, c) * (1 - abs(rng.gauss(0, 0.0035)))
        bars.append((o, hi, lo, c, rng.uniform(0.25, 1.0)))
        price = c

    # 마지막 종가를 헤더 현재가(87,300)에 맞춘다
    k = 87300.0 / bars[-1][3]
    bars = [(o * k, hi * k, lo * k, c * k, v) for o, hi, lo, c, v in bars]

    axis_w = 62
    plot_w = w - axis_w
    p_top, p_bot = 14, 330
    v_top, v_bot = 348, 404
    lo_all = min(b[2] for b in bars)
    hi_all = max(b[1] for b in bars)
    pad = (hi_all - lo_all) * 0.06
    lo_all -= pad
    hi_all += pad

    def y(p):
        return p_bot - (p - lo_all) / (hi_all - lo_all) * (p_bot - p_top)

    step = plot_w / n
    bw = step * 0.62
    parts = []

    # 1px 수평 격자 + 우측 가격 라벨
    for k in range(5):
        gp = lo_all + (hi_all - lo_all) * k / 4
        gy = y(gp)
        parts.append(f'<line x1="0" y1="{gy:.1f}" x2="{plot_w}" y2="{gy:.1f}" stroke="var(--k-line)" stroke-width="1"/>')
        parts.append(f'<text x="{w - 4}" y="{gy + 3.5:.1f}" text-anchor="end" class="ax">{fmt(gp)}</text>')

    # 볼륨
    v_max = max(b[4] for b in bars)
    for i, b in enumerate(bars):
        x = i * step + (step - bw) / 2
        vh = b[4] / v_max * (v_bot - v_top)
        parts.append(f'<rect x="{x:.1f}" y="{v_bot - vh:.1f}" width="{bw:.1f}" height="{vh:.1f}" fill="var(--k-line-strong)"/>')

    # 캔들 (상승 적 / 하락 청)
    for i, (o, hi, lo, c, _) in enumerate(bars):
        cx = i * step + step / 2
        x = i * step + (step - bw) / 2
        col = "var(--k-up)" if c >= o else "var(--k-down)"
        parts.append(f'<line x1="{cx:.1f}" y1="{y(hi):.1f}" x2="{cx:.1f}" y2="{y(lo):.1f}" stroke="{col}" stroke-width="1"/>')
        top, bot = sorted((y(o), y(c)))
        parts.append(f'<rect x="{x:.1f}" y="{top:.1f}" width="{bw:.1f}" height="{max(bot - top, 1):.1f}" fill="{col}"/>')

    # MA20 (전략 오버레이 — 앰버)
    pts = []
    for i in range(19, n):
        ma = sum(b[3] for b in bars[i - 19:i + 1]) / 20
        pts.append(f"{i * step + step / 2:.1f},{y(ma):.1f}")
    parts.append(f'<polyline points="{" ".join(pts)}" fill="none" stroke="var(--k-amber)" stroke-width="1.5"/>')

    # 전략 신호 마커 (앰버 = 시스템색)
    for idx, label in ((41, "진입"), (56, "진입"), (66, "청산")):
        cx = idx * step + step / 2
        my = y(bars[idx][2]) + 12
        tri = "▲" if label == "진입" else "▼"
        parts.append(f'<text x="{cx:.1f}" y="{my:.1f}" text-anchor="middle" class="mk">{tri}</text>')
        parts.append(f'<text x="{cx:.1f}" y="{my + 11:.1f}" text-anchor="middle" class="mklab">{label}</text>')

    # 하단 날짜 라벨 3개
    for frac, d in ((0.06, "2026-04-27"), (0.5, "2026-06-18"), (0.94, "2026-08-06")):
        parts.append(f'<text x="{plot_w * frac:.0f}" y="{h - 6}" text-anchor="middle" class="ax">{d}</text>')

    return (
        f'<svg viewBox="0 0 {w} {h}" class="mk-chart" preserveAspectRatio="none">'
        '<style>.ax{font:9.5px var(--k-mono);fill:var(--k-ink-3)}'
        '.mk{font:11px var(--k-sans);fill:var(--k-amber)}'
        '.mklab{font:8.5px var(--k-sans);fill:var(--k-amber)}</style>'
        + "".join(parts) + "</svg>"
    )


def line_svg(series, w=800, h=190, seed=11, x_labels=(), v_line=None, y_fmt="%"):
    """series: [(name, color, dash, points<float>[, x_offset])]."""
    n = max((s[4] if len(s) > 4 else 0) + len(s[3]) for s in series)
    lo = min(min(s[3]) for s in series)
    hi = max(max(s[3]) for s in series)
    pad = (hi - lo) * 0.12 or 1
    lo -= pad
    hi += pad
    p_top, p_bot, axis_w = 10, h - 22, 46
    plot_w = w - axis_w

    def y(v):
        return p_bot - (v - lo) / (hi - lo) * (p_bot - p_top)

    def x(i):
        return i / (n - 1) * plot_w

    parts = []
    for k in range(4):
        gv = lo + (hi - lo) * k / 3
        gy = y(gv)
        parts.append(f'<line x1="0" y1="{gy:.1f}" x2="{plot_w}" y2="{gy:.1f}" stroke="var(--k-line)" stroke-width="1"/>')
        lab = f"{gv:+.1f}%" if y_fmt == "%" else fmt(gv)
        parts.append(f'<text x="{w - 4}" y="{gy + 3.5:.1f}" text-anchor="end" class="ax">{lab}</text>')

    if v_line is not None:
        vx = x(v_line[0])
        parts.append(f'<line x1="{vx:.1f}" y1="{p_top}" x2="{vx:.1f}" y2="{p_bot}" stroke="var(--k-amber)" stroke-width="1" stroke-dasharray="3 3"/>')
        parts.append(f'<text x="{vx + 4:.1f}" y="{p_top + 9}" class="mklab">{v_line[1]}</text>')

    for s in series:
        name, color, dash, pts_v = s[0], s[1], s[2], s[3]
        off = s[4] if len(s) > 4 else 0
        pts = " ".join(f"{x(i + off):.1f},{y(v):.1f}" for i, v in enumerate(pts_v))
        d = f' stroke-dasharray="{dash}"' if dash else ""
        parts.append(f'<polyline points="{pts}" fill="none" stroke="{color}" stroke-width="1.5"{d}/>')

    for frac, d in x_labels:
        parts.append(f'<text x="{plot_w * frac:.0f}" y="{h - 6}" text-anchor="middle" class="ax">{d}</text>')

    return (
        f'<svg viewBox="0 0 {w} {h}" class="mk-chart" preserveAspectRatio="none">'
        '<style>.ax{font:9.5px var(--k-mono);fill:var(--k-ink-3)}'
        '.mklab{font:8.5px var(--k-sans);fill:var(--k-amber)}</style>'
        + "".join(parts) + "</svg>"
    )


def walk(n, seed, drift, vol, start=0.0):
    rng = random.Random(seed)
    v, out = start, []
    for _ in range(n):
        v += rng.gauss(drift, vol)
        out.append(v)
    return out


# ─────────────────────────────────────────────────────────────────
# 공용 셸 조각
# ─────────────────────────────────────────────────────────────────

WATCH_ROWS = [
    ("삼성전자", "005930", "87,300", "▲ +1.04%", "k-up", True),
    ("SK하이닉스", "000660", "312,500", "▼ -1.83%", "k-down", False),
    ("현대차", "005380", "268,000", "▲ +0.56%", "k-up", False),
    ("NAVER", "035420", "231,500", "▲ +2.21%", "k-up", False),
    ("카카오", "035720", "48,150", "▼ -0.72%", "k-down", False),
    ("셀트리온", "068270", "194,700", "— 0.00%", "k-flat", False),
    ("POSCO홀딩스", "005490", "402,000", "▼ -1.11%", "k-down", False),
    ("두산에너빌리티", "034020", "33,850", "▲ +3.20%", "k-up", False),
]


def topbar(active="리서치"):
    tabs = ""
    for name, toast in (("오늘", "프리셋 '내 가설의 오늘' — daily-open 화면"),
                        ("리서치", "프리셋 '리서치' — 스펙·차트·백테스트 비교"),
                        ("검증 기록", "사전등록·결과의 불변 이력")):
        on = " on" if name == active else ""
        tabs += f'<button class="k-toptab{on}" data-toast="{toast}">{name}</button>'
    return f"""
    <div class="k-topbar">
        <span class="k-logo">trdr</span>
        <nav class="k-topbar-tabs">{tabs}</nav>
        <div class="k-topbar-right">
            <span class="k-badge k-badge--live" data-toast="KIS 웹소켓 실시간 — 본인 키, 세션당 ~41종목">● KIS WS</span>
            <span class="k-badge" data-toast="금융위 API 일별 데이터 — D+1 보정 완료">금융위 D+1</span>
            <span class="k-clock">14:32:05 KST</span>
        </div>
    </div>"""


def watchlist():
    rows = ""
    for name, code, price, chg, cls, sel in WATCH_ROWS:
        rows += (
            f'<tr{" class=sel" if sel else ""} data-toast="{name} 차트로 전환">'
            f'<td>{name}<br><span class="k-dim" style="font:10px var(--k-mono)">{code}</span></td>'
            f'<td class="k-num">{price}</td><td class="k-num {cls}">{chg}</td></tr>'
        )
    return f"""
        <div class="k-pane mk-watch">
            <div class="k-pane-title">관심종목 <span class="k-badge k-badge--live">● LIVE</span></div>
            <div class="k-pane-body" style="overflow:auto">
            <table class="k-table">
                <tr><th>종목</th><th class="k-num">현재가</th><th class="k-num">등락</th></tr>
                {rows}
            </table>
            </div>
        </div>"""


def chart_pane():
    return f"""
        <div class="k-pane mk-center">
            <div class="k-pane-title">
                005930 삼성전자 · 일봉
                <span class="k-num" style="font-size:12px;color:var(--k-ink)">87,300</span>
                <span class="k-num k-up" style="font-size:11px">▲ +1.04%</span>
                <span style="margin-left:auto;display:flex;gap:6px">
                    <span class="k-badge k-badge--amber" data-toast="전략 오버레이 — strategies/momentum-3.trdr.yaml 저장 시 핫리로드">— MA20 · momentum-3 신호</span>
                    <span class="k-badge">KIS WS · LIVE</span>
                </span>
            </div>
            <div class="k-pane-body">
                {candles_svg()}
                <span class="mk-attrib" data-toast="lightweight-charts v5 Apache-2.0 — attribution 고지 상시 노출">TradingView Lightweight Charts™</span>
            </div>
        </div>"""


def strategy_pane():
    return """
        <div class="k-pane mk-strat">
            <div class="k-pane-title">전략 <span class="k-badge">strategies/*.trdr.yaml</span></div>
            <div class="k-pane-body" style="overflow:auto;padding:6px 0">
                <div class="mk-strow sel" data-toast="선택된 전략 — 차트 오버레이·명령줄 컨텍스트가 이 전략을 따른다">
                    <div class="mk-stname">momentum-3 <span class="k-dim">v4</span></div>
                    <div class="mk-stbadges">
                        <span class="k-badge k-badge--amber">사전등록 08-04</span>
                        <span class="k-badge">검증 D-23</span>
                    </div>
                </div>
                <div class="mk-strow" data-toast="초안 — 아직 사전등록 전, 자유 수정 가능">
                    <div class="mk-stname">meanrev-krx <span class="k-dim">v1</span></div>
                    <div class="mk-stbadges"><span class="k-badge">초안</span></div>
                </div>
                <div class="mk-strow" data-toast="판정 완료 — 장부에서 리포트 열람">
                    <div class="mk-stname">gap-open <span class="k-dim">v2</span></div>
                    <div class="mk-stbadges"><span class="k-badge">노이즈 07-30</span></div>
                </div>
                <hr class="k-hr" style="margin:6px 10px">
                <div class="mk-spec">
                    <div class="k-dim" style="font-size:10px;margin-bottom:4px">momentum-3.trdr.yaml — 미리보기</div>
                    <pre>universe: KOSPI200
signal:
  rank: momentum_20d
  top: 3
rebalance: monthly
cost:
  fee_bp: 1.5
  slip_bp: 5</pre>
                </div>
            </div>
        </div>"""


def terminal_pane():
    return """
        <div class="k-pane mk-term-pane">
            <div class="k-pane-title">터미널 — BYO 에이전트 <span class="k-badge">xterm · node-pty</span></div>
            <div class="k-term">
                <div><span class="t-prompt">❯</span> claude</div>
                <div class="t-out">✳ "20일 모멘텀 상위 3종목 월간 리밸런스" 를 스펙으로 형식화했습니다 → strategies/momentum-3.trdr.yaml</div>
                <div class="t-dim">  · universe: KOSPI200 · signal: momentum_20d top 3 · rebalance: monthly · cost 1.5+5bp</div>
                <div class="t-dim">⏺ trdr 파일워처: 차트 오버레이 갱신됨 (momentum-3 @ 005930)</div>
                <div class="t-out">✳ 탐색 결과가 그럴듯해도 결과는 등록 후 검증 구간에서 납니다. 등록으로 진행할까요?</div>
                <div><span class="t-prompt">❯</span> <span class="k-cmd-caret"></span></div>
            </div>
        </div>"""


def cmdline(inner, hint="⌘K 명령 · Tab 에이전트 모드", goto=None):
    g = f' data-goto="{goto}"' if goto else ""
    return f"""
    <div class="k-cmd"{g}>
        <span class="k-cmd-prompt">❯</span>
        {inner}
        <span class="k-cmd-hint">{hint}</span>
    </div>"""


def app(inner, cmd, extra="", active="리서치"):
    return f'<div class="k-app mk-desk">{topbar(active)}{inner}{cmd}{extra}</div>'


def cockpit_inner():
    return f"""
    <div class="mk-main">
        {watchlist()}
        {chart_pane()}
        {strategy_pane()}
    </div>
    {terminal_pane()}"""


# ─────────────────────────────────────────────────────────────────
# 화면 5개
# ─────────────────────────────────────────────────────────────────

def screen_cockpit():
    cmd = cmdline('<span class="k-dim">005930 — 명령 또는 에이전트 입력…</span>', goto="command")
    return f"""
    <section class="flow-step" data-screen="cockpit" data-title="콕핏 메인">
        <div class="step-head">콕핏 메인 <small>Electron + dockview · 진입 화면</small></div>
        <div class="device mk-device">{app(cockpit_inner(), cmd)}</div>
        <div class="step-note">기본 4-pane 레이아웃(관심종목·차트·전략·터미널). 레이아웃은 dockview 드래그 재배치·저장 전제 — 'r/unixporn식 내 레이아웃 공유' 루프의 기반. 명령줄을 누르면 팔레트가 열린다.</div>
    </section>"""


def screen_command():
    palette = """
        <div class="mk-palette-wrap">
        <div class="k-palette">
            <div class="k-palette-row" style="border-bottom:1px solid var(--k-line-strong);cursor:default">
                <span class="k-mode-chip on">CMD</span>
                <span class="k-mode-chip" data-toast="Tab — 같은 입력창이 그대로 에이전트 프롬프트가 된다 (원칙 4: 에이전트는 명령줄의 진화)">AGENT</span>
                <span class="k-palette-sub">Tab — 같은 입력창이 에이전트 프롬프트로 전환</span>
            </div>
            <div class="k-palette-row" data-toast="백테스트 실행 — trdr CLI의 backtest와 같은 동작 (동작 일치 원칙)">
                <span class="k-palette-mn">백테스트 <span class="k-dim">bt</span></span><span class="k-palette-desc">탐색 구간 백테스트 실행</span>
                <span class="k-palette-sub">선택 전략: momentum-3 v4</span>
            </div>
            <div class="k-palette-row sel" data-goto="preregister">
                <span class="k-palette-mn">등록 <span class="k-dim">reg</span></span><span class="k-palette-desc">사전등록 — 스펙 해시 고정</span>
                <span class="k-palette-sub">momentum-3 v4 · Enter</span>
            </div>
            <div class="k-palette-row" data-toast="차트 표시">
                <span class="k-palette-mn">차트 <span class="k-dim">ch</span></span><span class="k-palette-desc">종목 차트 표시</span>
            </div>
            <div class="k-palette-row" data-toast="수급 pane 열기 — 투자자별 순매수">
                <span class="k-palette-mn">수급 <span class="k-dim">flow</span></span><span class="k-palette-desc">투자자별 수급 보기</span>
            </div>
            <div class="k-palette-row" data-toast="관찰 상태 보기 — 검증 중 전략의 카운트다운·참고 지표">
                <span class="k-palette-mn">관찰 <span class="k-dim">obs</span></span><span class="k-palette-desc">검증 중 전략 관찰</span>
            </div>
        </div>
        </div>"""
    cmd = cmdline('<span class="k-cmd-token">005930</span> <span>등</span><span class="k-cmd-caret"></span>',
                  hint="한/영 모두 인식 · ↑↓ 선택 · Enter 실행")
    return f"""
    <section class="flow-step" data-screen="command" data-title="명령줄">
        <div class="step-head">명령줄 — 티커+단축 명령 <small>US-07 · 한국어 1차</small></div>
        <div class="device mk-device">{app(cockpit_inner(), cmd, palette)}</div>
        <div class="step-note">티커+단축 명령(005930 등록)이 기본 문법 — 한국어 1차, 영문 별칭 병기, 한영 오타 보정. 같은 입력창이 Tab으로 에이전트 프롬프트가 된다(원칙 4). 등록 행을 누르면 사전등록으로.</div>
    </section>"""


def screen_preregister():
    modal = """
    <div class="mk-dim"></div>
    <div class="mk-modal k-pane" style="width:680px">
        <div class="k-pane-title">사전등록 — momentum-3 v4 <span class="k-badge">등록</span></div>
        <div style="padding:16px 18px;display:flex;flex-direction:column;gap:10px">
            <dl style="margin:0">
                <div class="k-kv"><dt>스펙 파일</dt><dd class="k-num" style="font-size:11px">strategies/momentum-3.trdr.yaml</dd></div>
                <div class="k-kv"><dt>스펙 요약</dt><dd>KOSPI200 · momentum_20d 상위 3 · 월간 리밸런스 · 비용 1.5+5bp</dd></div>
                <div class="k-kv"><dt>고정되는 해시</dt><dd class="k-num k-amber-t" style="font-size:11px">sha256:9f3a4d2e…c21b</dd></div>
                <div class="k-kv"><dt>등록 시각</dt><dd class="k-num" style="font-size:11px">2026-08-04 09:12:31 KST</dd></div>
                <div class="k-kv"><dt>탐색 구간 (자유 실험)</dt><dd class="k-num" style="font-size:11px">2019-01-02 ~ 2025-12-30</dd></div>
            </dl>
            <hr class="k-hr" style="margin:2px 0">
            <div style="font-size:11px;font-weight:700;color:var(--k-ink-2);letter-spacing:.06em">검증 모드 선택</div>
            <div class="mk-mode-row" data-toast="과거 검증 — 탐색에 쓰지 않은 지나간 구간으로 즉시 검증. 결과가 바로 나온다">
                <span class="mk-radio"></span>
                <div><b>과거 검증</b> — 탐색에 쓰지 않은 2026-01-02 ~ 2026-07-31로 지금 바로 검증
                <div class="k-dim" style="font-size:10.5px">결과 즉시 · 결과에 "과거 검증" 라벨</div></div>
            </div>
            <div class="mk-mode-row sel" data-toast="실전 관찰 — 등록 후 실제 미래 데이터로 검증. 증거 무게가 가장 높다">
                <span class="mk-radio on"></span>
                <div><b>실전 관찰</b> — 2026-08-05부터 35 거래일, 실제 미래 데이터로 검증
                <div class="k-dim" style="font-size:10.5px">결과 2026-09-22 (D-0) · 그 전 지표는 참고용</div></div>
            </div>
            <div class="k-notice">등록은 전략을 <b>하나의 고정 해시</b>로 만들어 정직하게 테스트하도록 돕는 장치입니다. 등록 후 스펙을 고치면 새 등록이 생기고, 이전 등록과의 계보가 결과 리포트에 함께 표시됩니다.</div>
            <div style="display:flex;align-items:center;gap:16px;margin-top:4px">
                <div class="k-stamp k-stamp--sm">사전등록<small>REGISTERED</small></div>
                <span class="k-stamp-meta">이 도장이 결과 리포트까지 따라갑니다 —<br>해시·시각과 함께 재현 검증 가능</span>
                <span style="flex:1"></span>
                <button class="k-btn" data-goto="command">취소</button>
                <button class="k-btn k-btn--amber" data-goto="oos" data-toast="확정은 이 클릭뿐 — CLI·에이전트에는 확정 명령이 없다">등록 — 해시 고정</button>
            </div>
        </div>
    </div>"""
    cmd = cmdline('<span class="k-cmd-token">005930</span> <span>등록</span>', hint="")
    return f"""
    <section class="flow-step" data-screen="preregister" data-title="사전등록">
        <div class="step-head">사전등록 — 검증 모드 선택 <small>US-09·10·12</small></div>
        <div class="device mk-device">{app(cockpit_inner(), cmd, modal)}</div>
        <div class="step-note">고정되는 것(해시·시각·구간)을 전부 보여주고 검증 모드(과거 검증/실전 관찰)를 골라 확정한다. 확정 버튼은 GUI에만 있다 — 에이전트는 이 화면을 열고 채우기까지. 봉인·차단 어휘 없음.</div>
    </section>"""


def screen_oos():
    shadow = walk(12, seed=21, drift=0.21, vol=0.55)
    bench = walk(12, seed=22, drift=0.16, vol=0.4)
    svg = line_svg(
        [("전략 shadow", "var(--k-amber)", "", shadow), ("KOSPI200", "var(--k-ink-3)", "4 3", bench)],
        x_labels=((0.04, "08-05"), (0.5, "08-13"), (0.96, "08-21")),
    )
    inner = f"""
    <div class="mk-report">
        <div class="k-pane" style="width:880px">
            <div class="k-pane-title">관찰 — momentum-3 v4
                <span style="margin-left:auto;display:flex;gap:6px">
                    <span class="k-badge k-badge--amber">사전등록 08-04 09:12</span>
                    <span class="k-badge" data-toast="실전 관찰 모드 — 등록 후 실제 미래 데이터로 검증 중">실전 관찰</span>
                </span>
            </div>
            <div style="padding:18px 22px;display:flex;flex-direction:column;gap:14px">
                <div style="display:flex;align-items:center;gap:28px">
                    <div>
                        <div class="k-countdown">D-23</div>
                        <div class="k-countdown-label">결과까지 잔여 거래일 · 관측 12/35</div>
                    </div>
                    <div style="text-align:center">
                        <div class="k-stamp k-stamp--sm">사전등록<small>REGISTERED</small></div>
                        <div class="k-stamp-meta" style="margin-top:6px">sha256:9f3a…c21b · 2026-08-04</div>
                    </div>
                    <div class="k-notice" style="flex:1">결과 전 지표는 <b>참고용</b>입니다. 결과는 D-0에 한 번, 등록 때 정한 방법으로만 계산됩니다.</div>
                </div>
                <div>
                    <div style="display:flex;gap:14px;font-size:10.5px;margin-bottom:4px">
                        <span class="k-amber-t">— 전략 shadow</span>
                        <span class="k-dim">┄ KOSPI200</span>
                        <span class="k-dim" style="margin-left:auto">누적 수익률 · 관찰 시작 2026-08-05</span>
                    </div>
                    {svg}
                </div>
                <table class="k-table">
                    <tr><th>참고 지표 (판정 아님)</th><th class="k-num">전략 shadow</th><th class="k-num">KOSPI200</th></tr>
                    <tr><td>누적 수익률</td><td class="k-num k-up">▲ +2.31%</td><td class="k-num k-up">▲ +1.87%</td></tr>
                    <tr><td>추적 거래</td><td class="k-num">12건</td><td class="k-num k-dim">—</td></tr>
                    <tr><td>최대 낙폭</td><td class="k-num k-down">▼ -1.12%</td><td class="k-num k-down">▼ -0.94%</td></tr>
                </table>
            </div>
        </div>
    </div>"""
    cmd = cmdline('<span class="k-cmd-token">005930</span> <span>관찰</span>', hint="⌘K 명령 · Tab 에이전트 모드")
    return f"""
    <section class="flow-step" data-screen="oos" data-title="관찰">
        <div class="step-head">관찰 <small>US-11 · 카운트다운</small></div>
        <div class="device mk-device">{app(inner, cmd)}</div>
        <div class="step-note">기다림 자체를 UI로 만든다. 지표는 보이되 '참고용' 라벨이 강제되고, 결과는 D-0 하루 한 번. 다음 프레임은 D-0에 결과가 도착한 미래 시점.</div>
    </section>"""


def screen_verdict():
    is_eq = walk(46, seed=31, drift=0.5, vol=0.9)
    oos_eq = walk(35, seed=32, drift=-0.04, vol=0.8, start=is_eq[-1])
    svg = line_svg(
        [
            ("in-sample 누적", "var(--k-ink-2)", "", is_eq),
            ("out-of-sample", "var(--k-amber)", "", [is_eq[-1]] + oos_eq, 45),
        ],
        v_line=(45, "사전등록 08-04"),
        x_labels=((0.05, "2019"), (0.4, "2023"), (0.75, "2026"), (0.96, "판정")),
    )
    inner = f"""
    <div class="mk-report">
        <div class="k-pane" style="width:880px">
            <div class="k-pane-title">결과 리포트 — momentum-3 v4
                <span style="margin-left:auto;display:flex;gap:6px">
                    <span class="k-badge k-badge--amber" data-toast="검증 모드 라벨 — 과거 검증/실전 관찰의 증거 무게 차이를 표시">실전 관찰</span>
                    <span class="k-badge" data-toast="데이터 출처와 보정 시점이 모든 결과에 라벨로 남는다">데이터: 금융위 API 2019–2026 · D+1</span>
                    <span class="k-badge" data-toast="재현 검증 — 같은 입력이면 같은 결과가 재현된다는 해시 증거">재현 9f3a→77e0 ✓</span>
                    <span class="k-badge">재등록 이력 0회</span>
                </span>
            </div>
            <div style="padding:22px;display:flex;flex-direction:column;gap:16px">
                <div style="display:flex;align-items:center;gap:34px;padding:8px 0">
                    <div style="text-align:center;flex:none">
                        <div class="k-stamp">노이즈<small>NOISE · NOT SIGNIFICANT</small></div>
                        <div class="k-stamp-meta" style="margin-top:14px">p=0.41 · 결과 2026-09-22 15:40 KST<br>사전등록 sha256:9f3a…c21b (08-04)</div>
                    </div>
                    <div style="font-size:12px;line-height:1.7;color:var(--k-sec)">
                        탐색 구간 성과(sharpe 1.31)는 검증 구간 35 거래일에서 재현되지 않았습니다(sharpe 0.12).<br>
                        등록 때 정한 기준(유의수준 0.05)으로 <b class="k-amber-t">결과: 노이즈</b>입니다.<br>
                        <span class="k-dim">이 결과는 전략 폐기를 강제하지 않습니다 — 재등록하면 계보가 함께 남을 뿐입니다.</span>
                    </div>
                </div>
                {svg}
                <table class="k-table">
                    <tr><th>구간</th><th class="k-num">sharpe</th><th class="k-num">수익률(연)</th><th class="k-num">최대낙폭</th><th class="k-num">거래</th></tr>
                    <tr><td>탐색 구간 2019-01-02 ~ 2026-07-31</td><td class="k-num">1.31</td><td class="k-num k-up">▲ +18.4%</td><td class="k-num k-down">▼ -9.7%</td><td class="k-num">274회</td></tr>
                    <tr><td>검증 구간 2026-08-05 ~ 2026-09-22 (실전 관찰)</td><td class="k-num">0.12</td><td class="k-num k-up">▲ +1.9%</td><td class="k-num k-down">▼ -6.3%</td><td class="k-num">12회</td></tr>
                </table>
                <div style="display:flex;gap:8px">
                    <button class="k-btn k-btn--ghost" data-toast="결과 이미지 — 스탬프·해시·모드 라벨이 담긴 공유용 PNG (US-15)">결과 이미지 내보내기</button>
                    <button class="k-btn" data-toast="검증 기록으로 — 결과는 삭제·수정되지 않는다">검증 기록에 보관</button>
                    <span style="flex:1"></span>
                    <button class="k-btn" data-goto="cockpit">다음 가설로</button>
                </div>
            </div>
        </div>
    </div>"""
    cmd = cmdline('<span class="k-dim">다음 가설을 에이전트에게 —</span><span class="k-cmd-caret"></span>',
                  hint="Tab 에이전트 모드")
    return f"""
    <section class="flow-step" data-screen="verdict" data-title="결과 리포트">
        <div class="step-head">결과 리포트 — 노이즈 <small>US-14·15 · 실패 결과가 주인공</small></div>
        <div class="device mk-device">{app(inner, cmd)}</div>
        <div class="step-note">부정적 결과의 제품화 — 경쟁 공백 1번. 스탬프가 화면의 주인공이고, 축하도 위로도 없다. 결과 이미지가 '공유하고 싶은 실패 스크린샷'이 되는 게 목표. 헤더에 검증 모드 라벨 추가(2모드 결정 반영).</div>
    </section>"""


# ─────────────────────────────────────────────────────────────────
# 목업 전용 스타일 (mk-)
# ─────────────────────────────────────────────────────────────────

MK_CSS = """
/* ── 목업 전용 (mk-) — 데스크톱 프레임·레이아웃 ── */
.mk-device { width: 1280px; min-height: 800px; border-radius: 4px; background: var(--k-bg-0); }
.mk-desk { height: 800px; position: relative; }
.mk-main { display: flex; flex: 1; min-height: 0; }
.mk-watch { width: 240px; flex: none; border-top: none; border-left: none; }
.mk-center { flex: 1; min-width: 0; border-top: none; border-left: none; }
.mk-strat { width: 300px; flex: none; border-top: none; border-left: none; border-right: none; }
.mk-term-pane { height: 180px; flex: none; border-left: none; border-right: none; border-bottom: none; }
.mk-chart { display: block; width: 100%; height: auto; }
.mk-center .k-pane-body { padding: 6px 8px 0; }
.mk-attrib { position: absolute; left: 10px; bottom: 6px; font: 9px var(--k-sans); color: var(--k-ink-3); }
.mk-strow { padding: 7px 10px; border-bottom: 1px solid var(--k-line); cursor: pointer; }
.mk-strow.sel { background: var(--k-amber-bg); border-left: 2px solid var(--k-amber); }
.mk-stname { font-weight: 700; font-size: 12px; }
.mk-stbadges { display: flex; gap: 4px; margin-top: 3px; flex-wrap: wrap; }
.mk-spec { padding: 6px 10px; }
.mk-spec pre { font: 10.5px/1.55 var(--k-mono); color: var(--k-ink-2); background: var(--k-bg-0); border: 1px solid var(--k-line); padding: 8px 10px; overflow: hidden; }
.mk-palette-wrap { position: absolute; left: 0; right: 420px; bottom: 36px; }
.mk-dim { position: absolute; inset: 0; background: rgba(13, 13, 16, 0.66); }
.mk-modal { position: absolute; left: 50%; top: 50%; transform: translate(-50%, -50%); width: 620px; border-color: var(--k-line-strong); }
.mk-report { flex: 1; min-height: 0; display: flex; align-items: flex-start; justify-content: center; padding: 26px 0; overflow: hidden; background: var(--k-bg-0); }
.mk-mode-row { display: flex; gap: 10px; align-items: flex-start; padding: 8px 10px; border: 1px solid var(--k-line); cursor: pointer; font-size: 11.5px; }
.mk-mode-row.sel { border-color: var(--k-amber-dim); background: var(--k-amber-bg); }
.mk-radio { width: 12px; height: 12px; border: 1px solid var(--k-line-strong); border-radius: 50%; flex: none; margin-top: 2px; }
.mk-radio.on { border-color: var(--k-amber); box-shadow: inset 0 0 0 3px var(--k-bg-1); background: var(--k-amber); }
"""

PRESET_NOTES = """
    { screen: "cockpit", x: 50, y: 3, text: "원칙 1·2 — 앰버는 시스템(판정·명령·포커스) 전용, 시장 정보만 적/청. 배경은 3층 다크그레이, 순흑·glow·radius·그림자 없음." },
    { screen: "cockpit", x: 13, y: 22, text: "원칙 3·8 — 모든 숫자 tabular-nums·우측정렬(모노), 등락은 색+▲▼ 병기로 색 단독 의미 금지." },
    { screen: "cockpit", x: 88, y: 30, text: "전략의 정본은 파일(strategies/*.trdr.yaml). 에이전트가 파일을 쓰면 파일워처가 차트 오버레이·상태를 핫리로드 — 에이전트 비종속의 핵심." },
    { screen: "cockpit", x: 50, y: 78, text: "터미널 = BYO 에이전트(PTY). 사용자 구독의 Claude Code·Codex가 그대로 산다. 특정 에이전트 종속 없음." },
    { screen: "cockpit", x: 50, y: 96, text: "원칙 4 — 명령줄이 1급. 티커+니모닉 자리가 곧 에이전트 입력 자리." },
    { screen: "command", x: 30, y: 66, text: "블룸버그 함수 모델의 현대화 — 티커+단축 명령, 한국어 1차. Tab으로 같은 입력창이 에이전트 프롬프트가 된다: '에이전트는 명령줄의 진화'." },
    { screen: "preregister", x: 50, y: 42, text: "등록 = 자기 기만 방지 장치(강제 아님). 고정되는 것(해시·시각·구간)과 검증 모드(과거/실전)를 전부 보여주고 사람이 확정한다. 축하 없음." },
    { screen: "oos", x: 18, y: 32, text: "원칙 7 — 카운트다운. 기다림을 UI로 만들고, 결과 전 지표에는 '참고용' 라벨이 강제된다." },
    { screen: "verdict", x: 27, y: 33, text: "원칙 6 — 실패 결과(노이즈)가 시각 주인공. 경쟁 조사에서 '부정적 결과의 제품화'는 전 세계 공백. 헤더의 모드 라벨이 증거 무게를 구분한다." },
    { screen: "verdict", x: 75, y: 12, text: "판정의 신뢰 근거 3종을 헤더에 상시 노출: 데이터 provenance · replay 결정론 해시 · 재등록 이력." }
"""


# ─────────────────────────────────────────────────────────────────
# 조립
# ─────────────────────────────────────────────────────────────────

def main():
    html = TEMPLATE.read_text(encoding="utf-8")

    html = html.replace("<title>주제 이름 — 플로우 목업</title>", f"<title>{TITLE}</title>")
    html = html.replace('<div class="deck-title">주제 이름', f'<div class="deck-title">{TITLE.split(" — ")[0]}')

    kit_block = "/* ═══ [KIT] trdr ui-kit 인라인 (정본: artifacts/ui-kit/kit.css) ═══ */\n" + KIT_CSS + MK_CSS + "\n/* ═══ /[KIT] ═══ */"
    html = re.sub(r"/\* ═══ \[KIT\].*?/\[KIT\] ═══ \*/", lambda m: kit_block, html, flags=re.S)

    screens = "\n".join([
        screen_cockpit(),
        '<div class="flow-arrow" data-label="명령줄 포커스 (⌘K)"></div>',
        screen_command(),
        '<div class="flow-arrow" data-label="PREREG 실행"></div>',
        screen_preregister(),
        '<div class="flow-arrow" data-label="사전등록 — 해시 고정"></div>',
        screen_oos(),
        '<div class="flow-arrow" data-label="D-0 — 판정 도착 (미래 시점)"></div>',
        screen_verdict(),
    ])
    screens_block = "<!-- ═══ [SCREENS] ═══ -->\n" + screens + "\n<!-- ═══ /[SCREENS] ═══ -->"
    html = re.sub(r"<!-- ═══ \[SCREENS\].*?/\[SCREENS\] ═══ -->", lambda m: screens_block, html, flags=re.S)

    html = re.sub(
        r"var PRESET_NOTES = \[.*?\];",
        lambda m: "var PRESET_NOTES = [" + PRESET_NOTES + "];",
        html, flags=re.S,
    )
    html = html.replace('var MOCKUP_VERSION = "v1";', f'var MOCKUP_VERSION = "{VERSION}";')
    html = html.replace(
        "<li><b>v1</b> 최초 작성</li>",
        "<li><b>v1</b> 최초 작성</li>\n    <li><b>v2</b> PRD·용어집 반영 — 등록 화면을 검증 2모드(과거 검증/실전 관찰)로 개편, "
        "판정→결과 · OOS→검증/관찰 · 니모닉→단축 명령(한국어 1차), 봉인·차단 어휘 제거, CLI 우선 반영</li>",
    )

    OUT.write_text(html, encoding="utf-8")
    print(f"wrote {OUT} ({len(html):,} bytes)")


if __name__ == "__main__":
    main()
