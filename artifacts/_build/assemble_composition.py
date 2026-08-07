#!/usr/bin/env python3
"""trdr 목업 v4 — 화면 구성안(docs/SCREEN_COMPOSITION.md)을 그대로 그린다.

구조는 문서가 정본이고 여기서 재논의하지 않는다. 6프레임:
온보딩 1/3(수집기 연결·keychain 안내) → 2/3(데이터 셋업) → 착지 게이트
(리서치 빈 단계) → 성장(작업이 pane을 놓는다) → 리서치 완성형(리플레이
지평·등록 다리) → 오늘(결정론 답 스트립·전략 행).

근거 결정: 01kzdr2d·01kzdr99·01kzdwff·01kzdwfq·01kzdxn9·01kzdxng·
01kzdy9k·01kzdyrv·01kzdz3s. 공용 셸·SVG는 assemble_flow에서 가져온다.
"""
import pathlib
import random
import re

from assemble_flow import KIT_CSS, MK_CSS, TEMPLATE, cmdline, fmt, line_svg, walk

ROOT = pathlib.Path(__file__).resolve().parent.parent
OUT = ROOT / "2026-08-07_trdr-composition-flow.html"
TITLE = "trdr 화면 목업 — 구성안 v4"
VERSION = "v5"


# ─────────────────────────────────────────────────────────────────
# 공용 — v4 셸: 헤더에 지수 스트립 (결정 01kzdwfq)
# ─────────────────────────────────────────────────────────────────

def topbar(active="리서치", kis=True):
    tabs = ""
    for name, toast in (("리서치", "메인 화면 — 다음 가설을 어떻게 만드나"),
                        ("오늘", "등록 전략의 소비 화면 — 내 전략들에 무슨 일이 있나")):
        on = " on" if name == active else ""
        tabs += f'<button class="k-toptab{on}" data-toast="{toast}">{name}</button>'
    tabs += ('<button class="k-toptab" style="color:var(--k-ink-3)" '
             'data-toast="지금 배치를 이름 붙여 저장 — 내 배치 탭이 생긴다">내 배치＋</button>')
    if kis:
        strip = ('<span class="mk4-idx" data-toast="지수는 pane이 아니라 헤더다 — 어느 화면에서든 유효한 상시 정보 (결정 01kzdwfq). '
                 '온보딩 직후 여기 지수가 뜨는 것이 연결 증명이다">'
                 'KOSPI <b class="k-num k-up">2,634.27 ▲0.8%</b>'
                 '&nbsp;&nbsp;KOSDAQ <b class="k-num k-down">771.44 ▼0.3%</b></span>'
                 '<span class="k-badge k-badge--amber" data-toast="장전·장중·마감">장중</span>')
    else:
        strip = ('<span class="mk4-idx k-dim" data-toast="KIS 수집기를 연결하면 지수가 표시된다 — 연결할 이유의 상시 노출">'
                 'KIS 연결 시 지수 표시</span>')
    return f"""
    <div class="k-topbar">
        <span class="k-logo">trdr</span>
        <nav class="k-topbar-tabs">{tabs}</nav>
        <div class="k-topbar-right">{strip}<span class="k-clock">09:04:11</span></div>
    </div>"""


def app(inner, cmd, extra="", active="리서치", banner="", kis=True):
    return (f'<div class="k-app mk-desk">{topbar(active, kis)}{banner}'
            f'<div class="k-canvas">{inner}</div>{cmd}{extra}</div>')


def frame(sid, title, small, device, note):
    return f"""
    <section class="flow-step" data-screen="{sid}" data-title="{title}">
        <div class="step-head">{title} <small>{small}</small></div>
        <div class="device mk-device">{device}</div>
        <div class="step-note">{note}</div>
    </section>"""


def arrow(label):
    return f'<div class="flow-arrow" data-label="{label}"></div>'


def cmd_global(inner=None, hint=None):
    """전역 명령줄 (결정 01kzdy9k) — 앱에게 말하는 곳."""
    if inner is None:
        inner = '<span class="k-dim">005930 · 목록 · 전략 …</span>'
    if hint is None:
        hint = "명령줄은 앱에게 · 에이전트와는 터미널에서"
    return cmdline(inner, hint=hint)


def candles_small(w=900, h=200, n=72, seed=9):
    """작은 pane용 캔들 — 전 좌표를 w·h 비례로 계산한다.

    assemble_flow.candles_svg는 볼륨 밴드·라벨 y를 하드코딩해 h>=410에서만
    성립한다 (v4에서 h=340을 넘겼다가 하단이 잘렸다). 여기서는 볼륨을 빼고
    비례 좌표만 쓴다.
    """
    rng = random.Random(seed)
    price, bars = 78000.0, []
    for i in range(n):
        o = price
        drift = 0.0016 if i > n * 0.5 else -0.0004
        c = o * (1 + rng.gauss(drift, 0.011))
        hi = max(o, c) * (1 + abs(rng.gauss(0, 0.0035)))
        lo = min(o, c) * (1 - abs(rng.gauss(0, 0.0035)))
        bars.append((o, hi, lo, c))
        price = c
    k = 87300.0 / bars[-1][3]
    bars = [(o * k, hi * k, lo * k, c * k) for o, hi, lo, c in bars]

    axis_w, p_top = 62, 10
    p_bot = h - 22
    plot_w = w - axis_w
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
    for g in range(4):
        gp = lo_all + (hi_all - lo_all) * g / 3
        gy = y(gp)
        parts.append(f'<line x1="0" y1="{gy:.1f}" x2="{plot_w}" y2="{gy:.1f}" stroke="var(--k-line)" stroke-width="1"/>')
        parts.append(f'<text x="{w - 4}" y="{gy + 3.5:.1f}" text-anchor="end" class="ax">{fmt(gp)}</text>')
    for i, (o, hi, lo, c) in enumerate(bars):
        cx = i * step + step / 2
        x = i * step + (step - bw) / 2
        col = "var(--k-up)" if c >= o else "var(--k-down)"
        parts.append(f'<line x1="{cx:.1f}" y1="{y(hi):.1f}" x2="{cx:.1f}" y2="{y(lo):.1f}" stroke="{col}" stroke-width="1"/>')
        top, bot = sorted((y(o), y(c)))
        parts.append(f'<rect x="{x:.1f}" y="{top:.1f}" width="{bw:.1f}" height="{max(bot - top, 1.2):.1f}" fill="{col}"/>')
    pts = []
    for i in range(19, n):
        ma = sum(b[3] for b in bars[i - 19:i + 1]) / 20
        pts.append(f"{i * step + step / 2:.1f},{y(ma):.1f}")
    parts.append(f'<polyline points="{" ".join(pts)}" fill="none" stroke="var(--k-amber)" stroke-width="1.5"/>')
    for frac, label in ((0.57, "진입"), (0.78, "진입"), (0.92, "청산")):
        idx = int(n * frac)
        cx = idx * step + step / 2
        my = y(bars[idx][2]) + 11
        tri = "▲" if label == "진입" else "▼"
        parts.append(f'<text x="{cx:.1f}" y="{my:.1f}" text-anchor="middle" class="mk">{tri}</text>')
        parts.append(f'<text x="{cx:.1f}" y="{my + 10:.1f}" text-anchor="middle" class="mklab">{label}</text>')
    for frac, d in ((0.06, "2026-04-27"), (0.5, "2026-06-18"), (0.94, "2026-08-06")):
        parts.append(f'<text x="{plot_w * frac:.0f}" y="{h - 6}" text-anchor="middle" class="ax">{d}</text>')
    return (
        f'<svg viewBox="0 0 {w} {h}" class="mk-chart" preserveAspectRatio="none">'
        '<style>.ax{font:9.5px var(--k-mono);fill:var(--k-ink-3)}'
        '.mk{font:11px var(--k-sans);fill:var(--k-amber)}'
        '.mklab{font:8.5px var(--k-sans);fill:var(--k-amber)}</style>'
        + "".join(parts) + "</svg>"
    )


def term_pane(lines, lead=True, style="flex:1"):
    cls = " k-pane--lead k-focus" if lead else ""
    body = "".join(lines)
    return f"""
        <div class="k-pane{cls} mk-term-pane" style="{style}">
            <div class="k-pane-title">터미널 — 에이전트와 대화하는 곳
                <span class="k-badge" style="margin-left:auto"
                      data-toast="xterm — BYO 에이전트 CLI의 자체 TUI가 곧 대화 화면이다. trdr는 창만 제공한다">xterm</span></div>
            <div class="k-term">{body}</div>
        </div>"""


# ─────────────────────────────────────────────────────────────────
# 온보딩 셸 — 다이얼로그가 아니라 전체 화면 (v5 · 핀 #1 · 결정 01kze1r7)
# 1/3의 본문은 설정 > 수집기 섹션과 같은 표면을 재사용한다.
# ─────────────────────────────────────────────────────────────────

def wizard_shell(step, main, side, foot_note, foot_btn):
    steps = ""
    for i, name in ((1, "수집기 연결"), (2, "데이터 셋업"), (3, "에이전트 연결")):
        on = " on" if i == step else ""
        done = " done" if i < step else ""
        toast = ("3/3은 별도 화면이 아니라 착지 화면의 게이트다 — 에이전트의 첫 CLI 호출 감지로 완료"
                 if i == 3 else f"처음 설정 {i}단계")
        steps += (f'<span class="mk4-wstep{on}{done}" data-toast="{toast}">'
                  f'<b>{i}</b> {name}</span>')
    return f"""
    <div class="k-app mk-desk">
        <div class="mk4-fullhead">
            <span class="k-logo">trdr</span>
            <span class="mk4-step">처음 설정</span>
            <div class="mk4-wsteps">{steps}</div>
        </div>
        <div class="mk4-fullbody">
            <div class="mk4-fullmain">{main}</div>
            <aside class="mk4-fullside">{side}</aside>
        </div>
        <div class="mk4-fullfoot">
            <span class="k-dim" style="font-size:10.5px">{foot_note}</span>
            {foot_btn}
        </div>
    </div>"""


# ─────────────────────────────────────────────────────────────────
# 1. 온보딩 1/3 — 수집기 연결 (결정 01kzdz3s)
# ─────────────────────────────────────────────────────────────────

COLLECTORS = [
    dict(name="KIS", req=True, data="시세·실시간 (웹소켓 ~41슬롯) · 수급(오늘부터 축적)",
         state="ok", note="✓ 연결 확인 — 테스트 호출 통과",
         toast="검증은 저장이 아니라 테스트 호출이다 — '저장됨'이 아니라 '연결됨'을 확인하고 넘어간다"),
    dict(name="DART", req=False, data="공시", state="input", note="키 입력 후 [확인]",
         toast="권장 — 지금 연결하거나 나중에 설정에서"),
    dict(name="금융위", req=False, data="보조 데이터", state="later", note="나중에 설정에서 연결 가능",
         toast="권장 — 스킵해도 시작할 수 있다"),
]


def collector_surface():
    """수집기 연결 표면 — 온보딩 1/3과 설정 > 수집기가 같은 것을 렌더링한다."""
    rows = ""
    for c in COLLECTORS:
        req = ('<span class="k-badge k-badge--amber">필수</span>' if c["req"]
               else '<span class="k-badge">권장</span>')
        if c["state"] == "ok":
            field = ('<span class="mk4-key k-num">●●●●●●●●●●●●</span> '
                     '<span class="k-badge k-badge--amber">✓ 테스트 호출 통과</span>')
        elif c["state"] == "input":
            field = ('<span class="mk4-key k-num">dart-</span><span class="k-cmd-caret"></span> '
                     '<button class="k-btn" data-toast="테스트 호출로 검증 — 통과해야 연결됨으로 표시된다">확인</button>')
        else:
            field = '<span class="k-dim" style="font-size:11px">비워 두면 설정에서 언제든</span>'
        rows += f"""
            <div class="mk4-colrow" data-toast="{c['toast']}">
                <span class="mk4-colname">{c['name']} {req}</span>
                <span class="mk4-coldata">{c['data']}</span>
                <span class="mk4-colkey">{field}</span>
            </div>"""
    return f"""
        <div class="mk4-sect" style="margin-top:0">내장 수집기 — 키는 각 기관에서 무료로 발급합니다</div>
        {rows}
        <div class="mk4-colrow mk4-plugin"
             data-toast="수집은 플러그인 표면이다 (결정 01kzdr99) — 내장 3개도 같은 표면 위의 플러그인. KRX처럼 id/pw 로그인이 필요한 채널은 공식 지원하지 않는다 (결정 01kzdr2d)">
            <span class="mk4-colname k-dim">＋</span>
            <span class="mk4-coldata k-dim">서드파티 수집기를 설치하면 여기에 나타납니다</span>
            <span class="mk4-colkey"></span>
        </div>"""


def screen_onboard1():
    side = """
        <div class="mk4-trust"
             data-toast="키를 건네는 바로 그 순간에 local-first 신뢰 명제를 말한다 (결정 01kzdz3s)">
            🔒 모든 키는 <b>이 기기의 keychain에만</b> 저장됩니다.
            어떤 서버로도 전송되지 않고, 화면과 로그에도 값이 표시되지 않습니다.
        </div>
        <div class="mk4-sidebox">
            <div class="mk4-sect" style="margin-top:0">키 발급 안내</div>
            <div class="mk4-siderow" data-toast="외부 브라우저로 연다">KIS — 한국투자 개발자센터 ↗</div>
            <div class="mk4-siderow" data-toast="외부 브라우저로 연다">DART — 오픈API 인증키 ↗</div>
            <div class="mk4-siderow" data-toast="외부 브라우저로 연다">금융위 — 공공데이터포털 ↗</div>
        </div>
        <div class="mk4-sidebox" style="font-size:11px;line-height:1.7;color:var(--k-ink-3)"
             data-toast="동작 일치 — 온보딩과 설정이 같은 표면을 렌더링한다 (결정 01kze1r7). 나중에 키를 더하러 오는 곳이 이 화면이다">
            이 화면은 나중에 <b>설정 → 수집기</b>에서 그대로 다시 만납니다.
            지금 스킵한 수집기는 거기서 연결합니다.
        </div>"""
    device = wizard_shell(
        1, collector_surface(), side,
        "필수(KIS)가 검증되면 넘어갈 수 있습니다",
        '<button class="k-btn k-btn--amber" data-goto="onboard2">다음 — 데이터 셋업</button>')
    return frame(
        "onboard1", "온보딩 1/3 — 수집기 연결", "VW-ONBOARD · 결정 01kzdz3s·01kzdr99·01kze1r7",
        device,
        "실행 직후의 첫 화면 — 다이얼로그가 아니라 전체 화면이고, 본문(수집기 표면)은 "
        "<b>설정 → 수집기 섹션과 같은 컴포넌트</b>다: 온보딩은 그 표면의 첫 노출이고 설정이 상주 "
        "자리다(핀 #1 반영, 결정 01kze1r7). 기본 데이터 수집용 키 연결이 모든 것에 선행한다 — "
        "KIS만 필수이고, 검증은 저장이 아니라 <b>테스트 호출</b>이다. 화면은 고정 표가 아니라 "
        "설치된 수집기 레지스트리를 렌더링한다: 서드파티 수집기를 설치하면 같은 화면에 나타난다. "
        "keychain 안내는 우측 레일의 고정 요소다 — 키를 건네는 바로 그 순간이 local-first를 "
        "말할 자리이기 때문이다.")


# ─────────────────────────────────────────────────────────────────
# 2. 온보딩 2/3 — 데이터 셋업
# ─────────────────────────────────────────────────────────────────

def screen_onboard2():
    bundled = ""
    for name, desc in (("상폐 명부", "4,052행 · 1990~ — 상폐 포함이 데이터의 차별 요소"),
                       ("종목 마스터", "전 상장 종목 코드·이름·시장"),
                       ("거래일 캘린더", "휴장일 포함")):
        bundled += f"""
            <div class="mk4-colrow" data-toast="재배포 가능한 데이터만 설치에 들어간다 — 선택이 아니라 자동 포함">
                <span class="mk4-colname">{name}</span>
                <span class="mk4-coldata">{desc}</span>
                <span class="mk4-colkey"><span class="k-badge">포함됨</span></span>
            </div>"""
    personal = ""
    for name, desc, toast in (
            ("KIS 일봉 백필", "1,900+ 종목 — 과거로 채워 들어감",
             "백필은 백그라운드로 계속된다 — 완료는 온보딩의 책임이 아니다"),
            ("KIS 수급 축적", "오늘부터 쌓임 — 연결 시점 이후가 내 데이터",
             "수급 이력은 개인 수집이다 — 재배포할 수 없는 데이터라 번들에 없다"),
            ("DART 공시 수집", "연결한 경우 — 최신부터",
             "1/3에서 연결한 수집기만 시작된다")):
        personal += f"""
            <div class="mk4-colrow" data-toast="{toast}">
                <span class="mk4-colname">{name}</span>
                <span class="mk4-coldata">{desc}</span>
                <span class="mk4-colkey"><span class="k-badge k-badge--amber">시작 대기</span></span>
            </div>"""

    main = f"""
        <div class="mk4-sect" style="margin-top:0">기본 포함 — 설치에 이미 들어 있습니다</div>
        {bundled}
        <div class="mk4-sect">개인 수집 — 내 키로 시작합니다</div>
        {personal}"""
    side = """
        <div class="mk4-trust" style="border-color:var(--k-line)">
            백필 완료를 기다리지 않습니다. 진행률은 진입 후 <b>수집 상태</b> pane이 보여줍니다.
        </div>
        <div class="mk4-sidebox" style="font-size:11px;line-height:1.7;color:var(--k-ink-3)"
             data-toast="재배포 가능한 데이터만 번들에 들어간다 — 수급 이력처럼 재배포할 수 없는 것은 내 키로 내가 쌓는다">
            기본 포함은 재배포 가능한 데이터입니다.
            차별 요소일수록 <b>내 키로 쌓는 내 데이터</b>입니다.
        </div>"""
    device = wizard_shell(
        2, main, side,
        "시작 후 언제든 설정에서 수집기를 더할 수 있습니다",
        '<button class="k-btn k-btn--amber" data-goto="landing">수집 시작하고 진입</button>')
    return frame(
        "onboard2", "온보딩 2/3 — 데이터 셋업", "VW-ONBOARD · PRD §4.2",
        device,
        "기본 제공과 개인 수집의 분기점. 재배포 가능한 데이터(상폐 명부 등)는 설치에 자동 포함이라 "
        "보여주기만 하고, 재배포할 수 없는 데이터(수급 이력)는 내 키로 내가 쌓는다 — 배포 vs 개인 "
        "수집이 내장 vs 플러그인 구도와 겹치는 지점이다. [수집 시작]이 백필을 시작하며 앱 셸로 "
        "진입한다. 위저드는 여기서 끝난다 — 백필 완료까지 잡아두면 이탈 지점이 된다.")


# ─────────────────────────────────────────────────────────────────
# 3. 착지 게이트 — 리서치 빈 단계 (3/3)
# ─────────────────────────────────────────────────────────────────

def collect_pane(style="width:360px;flex:none", progress=34):
    return f"""
        <div class="k-pane" style="{style}">
            <div class="k-pane-title">데이터가 어디까지 있나 <span class="k-badge">PN-DATA</span></div>
            <div class="k-pane-body" style="padding:14px">
                <div class="mk4-progrow" data-toast="백필은 과거로 채워 들어간다 — 언제부터 백테스트 가능한지가 이 pane의 답">
                    <span>일봉 백필</span>
                    <div class="mk4-prog"><i style="width:{progress}%"></i></div>
                    <span class="k-num">{progress}%</span>
                </div>
                <div class="k-kv"><dt>백테스트 가능 구간</dt><dd class="k-num">2019-03 ~ 오늘</dd></div>
                <div class="k-kv"><dt>수급 축적</dt><dd class="k-num">오늘 시작 · D+0</dd></div>
                <div class="k-kv"><dt>공시</dt><dd class="k-num">최신 08-07 08:41</dd></div>
                <hr class="k-hr" style="margin:12px 0">
                <div style="font-size:11px;line-height:1.7;color:var(--k-ink-2)">
                    수집은 백그라운드로 계속됩니다. 백테스트 가능 구간이
                    과거로 늘어납니다.
                </div>
            </div>
        </div>"""


def screen_landing():
    banner = """
    <div class="mk4-banner"
         data-toast="온보딩 3/3 — 별도 화면이 아니라 착지 화면에 걸린 게이트다. 에이전트의 첫 trdr CLI 호출을 감지하면 이 배너가 사라진다 = 온보딩 완료 (결정 01kzdz3s)">
        <b>시작하려면 터미널에서 에이전트를 실행하세요</b> — 예) <span class="k-num">claude</span> · <span class="k-num">codex</span>.
        첫 가설 만들기는 에이전트와 합니다.
        <span class="k-dim" style="margin-left:auto;font-size:10.5px">에이전트가 처음 trdr를 부르면 이 안내는 사라집니다</span>
    </div>"""
    inner = f"""
    {collect_pane()}
    {term_pane([
        '<div><span class="t-prompt">❯</span> <span class="k-cmd-caret"></span></div>',
        '<div class="t-dim" style="margin-top:14px">여기는 셸입니다 — 쓰던 에이전트를 그대로 실행하세요.<br>'
        'claude · codex · 무엇이든. 에이전트는 trdr CLI로 이 화면과 데이터를 읽고 조작합니다.</div>',
    ])}"""
    cmd = cmd_global(
        hint="명령줄은 앱에게 — 자연어를 치면 '에이전트와는 터미널에서'라고 안내합니다")
    return frame(
        "landing", "착지 — 리서치 빈 단계 + 게이트", "온보딩 3/3 · SH-RESEARCH · 결정 01kzdwff·01kzdz3s",
        app(inner, cmd, banner=banner),
        "온보딩이 끝나면 도착하는 첫 화면 — 리서치 프리셋의 빈 단계다. 헤더에 지수가 떠 있는 것이 "
        "1/3 연결의 증명이고(실시간이라 백필을 기다리지 않는다), 수집 상태 pane이 대기의 이유를 "
        "설명한다. 빈 차트·빈 백테스트를 미리 깔지 않는다 — 화면의 모든 pane이 자기 질문에 답하고 "
        "있어야 하기 때문이다(성장 규칙). 에이전트 연결은 위저드 단계가 아니라 이 화면의 게이트다: "
        "완료 조건이 '에이전트의 첫 CLI 호출 감지'로 검증 가능하다. 화면을 잠그지는 않는다 — 첫 "
        "행동(가설 만들기)이 에이전트 경유라서, 에이전트 없이는 어차피 시작이 성립하지 않는다.")


# ─────────────────────────────────────────────────────────────────
# 4. 성장 — 작업이 pane을 놓는다
# ─────────────────────────────────────────────────────────────────

def spec_pane(focus=False, style="flex:1"):
    cls = " k-focus" if focus else ""
    return f"""
        <div class="k-pane{cls}" style="{style}">
            <div class="k-pane-title">momentum-3 — 초안 <span class="k-badge">수정 중</span>
                <span class="k-badge" style="margin-left:auto"
                      data-toast="파일이 정본이다 — strategies/momentum-3.trdr.yaml. 에이전트가 저장하면 이 pane이 자동으로 놓인다">PN-SPEC</span></div>
            <div class="k-pane-body" style="padding:12px 14px;font-size:11.5px;line-height:1.8">
                <div class="k-kv"><dt>유니버스</dt><dd>KOSPI200 · 거래대금 상위 100</dd></div>
                <div class="k-kv"><dt>진입</dt><dd>20일 모멘텀 상위 3종목</dd></div>
                <div class="k-kv"><dt>청산</dt><dd>순위 이탈 시 익일 시가</dd></div>
                <div class="k-kv"><dt>리밸런스</dt><dd>매일</dd></div>
                <div class="k-kv"><dt>비용</dt><dd>거래세·수수료·호가단위 모델</dd></div>
            </div>
        </div>"""


def screen_growth():
    inner = f"""
    <div class="k-col" style="flex:1">
        {spec_pane(focus=True)}
        {collect_pane(style="height:200px;flex:none", progress=41)}
    </div>
    {term_pane([
        '<div><span class="t-prompt">❯</span> claude</div>',
        '<div class="t-out">✳ trdr 작업 폴더입니다. 수집 상태를 확인했습니다 — 백테스트 가능 구간 2019-03~.</div>',
        '<div style="margin-top:10px"><span class="t-prompt">›</span> 외국인 순매수 붙는 쪽으로 모멘텀 가설 하나 잡아줘</div>',
        '<div class="t-out">✳ 20일 모멘텀 상위 3종목 유지 전략 초안을 스펙 파일로 저장했습니다 → '
        'strategies/momentum-3.trdr.yaml. 스펙 pane이 화면에 놓였습니다. 탐색 백테스트를 돌릴까요?</div>',
        '<div style="margin-top:10px"><span class="t-prompt">›</span> 돌려봐 <span class="k-cmd-caret"></span></div>',
    ], style="width:430px;flex:none")}"""
    cmd = cmd_global()
    return frame(
        "growth", "성장 — 작업이 pane을 놓는다", "SH-RESEARCH · 결정 01kzdwff",
        app(inner, cmd),
        "배너가 사라졌다 — 에이전트가 연결됐다. 에이전트가 스펙 파일을 저장하자 스펙 pane이 "
        "자동으로 놓였다(앰버 테두리 = 방금 놓임). 빈 pane을 미리 깔지 않으므로 화면은 작업이 "
        "진행된 만큼만 자란다 — pane 배치 모델(치면·만들면 나타난다)을 첫 세션이 실제 동작으로 "
        "가르치는 지점이다. 차트와 백테스트도 필요해지는 순간에 각자의 경로로 놓인다.")


# ─────────────────────────────────────────────────────────────────
# 5. 리서치 완성형 — 리플레이 지평 · 등록 다리
# ─────────────────────────────────────────────────────────────────

def horizon_strip():
    cells = ""
    for label, val, cls in (("1일", "+0.4%", "k-up"), ("7일", "+1.9%", "k-up"),
                            ("1개월", "+6.2%", "k-up"), ("1년", "+18.4%", "k-up")):
        cells += (f'<span class="mk4-hz" data-toast="그 시점에 등록했다면 — 이후 {label} 리플레이 성적 (결정 01kzdxng)">'
                  f'<i>{label}</i><b class="k-num {cls}">{val}</b></span>')
    return cells


def screen_research():
    series = [("전략", "var(--k-amber)", "", walk(120, 5, 0.16, 1.05)),
              ("KOSPI", "var(--k-ink-3)", "3 3", walk(120, 11, 0.05, 0.7))]
    bt_curve = line_svg(series, w=860, h=160, v_line=(36, "등록 시점으로 가정"),
                        x_labels=((0.08, "2025-09"), (0.5, "2026-02"), (0.92, "2026-08")))
    inner = f"""
    <div class="k-col" style="flex:1">
        {spec_pane(style="flex:1")}
        <div class="k-pane" style="flex:1">
            <div class="k-pane-title">005930 삼성전자 · 일봉 <span class="k-badge k-badge--live">● 실시간</span>
                <span class="k-badge" style="margin-left:auto">PN-CHART</span></div>
            <div class="k-ans" data-toast="답이 수치인 pane은 답 스트립을 갖는다 — 4층 구조 (§4.1)">
                <span class="k-ans-fig">87,300</span>
                <span class="k-ans-delta k-up">▲ +1.04%</span>
                <span class="k-ans-sub">스펙의 진입 표식이 캔들 위에 있습니다</span>
            </div>
            <div class="k-pane-body">{candles_small(w=900, h=185, seed=9)}</div>
        </div>
        <div class="k-pane" style="flex:1.15">
            <div class="k-pane-title">이 스펙이 과거에 통했나 <span class="k-badge">탐색 — 근거 아님</span>
                <span class="k-badge" style="margin-left:auto"
                      data-toast="탐색 백테스트 — 몇 번이고 돌려볼 수 있다. 근거가 되는 것은 등록 후 out-of-sample뿐">PN-BT</span></div>
            <div class="k-ans" data-toast="백테스트 pane의 답 스트립 = 리플레이 지평 뷰. 과거 한 시점을 '그때 등록했다면'으로 잡고 지평별 성적을 본다 (결정 01kzdxng)">
                {horizon_strip()}
            </div>
            <div class="k-pane-body" style="padding:8px 10px">{bt_curve}</div>
            <div class="mk4-actbar">
                <span class="k-dim" style="font-size:10.5px">거래 214회 · 벤치마크 KOSPI +9.1%</span>
                <button class="k-btn k-btn--amber"
                        data-toast="생산→소비 다리 (§7) — 등록 pane이 열린다: 무엇을 걸고 확정하나. 확정하면 오늘 탭에 행이 생긴다">이 스펙으로 등록</button>
            </div>
        </div>
    </div>
    {term_pane([
        '<div><span class="t-prompt">›</span> 돌려봐</div>',
        '<div class="t-out">✳ 탐색 백테스트 완료 — 2025-09부터 잡으면 1년 +18.4%, 벤치마크 +9.1%. '
        '등록 시점을 어디로 가정하느냐에 따라 1일·7일 성적이 흔들립니다. 백테스트 pane을 놓았습니다.</div>',
        '<div style="margin-top:10px"><span class="t-prompt">›</span> 시점 몇 개 더 바꿔서 확인해줘 <span class="k-cmd-caret"></span></div>',
        '<div class="t-dim" style="margin-top:14px">탐색은 자유입니다 — 몇 번을 돌려도 좋습니다. '
        '근거가 되는 것은 등록 뒤의 out-of-sample 구간뿐입니다.</div>',
    ], style="width:430px;flex:none")}"""
    cmd = cmd_global()
    return frame(
        "research", "리서치 완성형 — 실험과 등록 다리", "SH-RESEARCH §5.2 · 결정 01kzdxng",
        app(inner, cmd),
        "메인 화면의 완성형. 터미널이 리드다 — 화면의 질문 '다음 가설을 어떻게 만드나'의 답은 "
        "에이전트의 지금 작업이기 때문이다. 좌열은 산출 순서(스펙→차트→백테스트)대로 위→아래. "
        "백테스트의 답 스트립이 곧 리플레이 지평 뷰다: 과거 한 시점을 '그때 등록했다면'으로 잡고 "
        "1일·7일·1개월·1년 성적을 본다. 어느 정도 실험한 뒤의 다음 행동이 pane의 액션 층에 있다 — "
        "[이 스펙으로 등록]. 이 다리가 생산(리서치)과 소비(오늘)를 잇고, 등록 확정의 순간 저쪽 "
        "화면에 행이 생긴다.")


# ─────────────────────────────────────────────────────────────────
# 6. 오늘 — 결정론 답 스트립 · 전략 행
# ─────────────────────────────────────────────────────────────────

STRATS = [
    dict(name="gap-open", ver="v2", hold="— 청산 완료", ret="+1.8%", rcls="k-flat",
         n="41", bm="+3.4%", bmcls="k-up", when="종료 07-30",
         badge='<span class="k-stamp k-stamp--sm">노이즈</span>',
         toast="판정 도착 행 — 누르면 결과 리포트가 열린다. 리포트의 액션 층에 '이 결과에서 새 가설'(소비→생산 다리)"),
    dict(name="momentum-3", ver="v4", hold="SK하이닉스 외 2", ret="+6.2%", rcls="k-up",
         n="12", bm="+2.1%", bmcls="k-up", when="D-23",
         badge='<span class="k-badge mk4-run">관찰 중</span>',
         toast="관찰 중 행 — 누르면 하단 한 자리에 관찰 pane이 SWAP: 가상 포지션 상세와 수익 곡선"),
    dict(name="meanrev-krx", ver="v1", hold="셀트리온", ret="-0.4%", rcls="k-down",
         n="3", bm="+2.1%", bmcls="k-up", when="D-71",
         badge='<span class="k-badge mk4-run">관찰 중</span>',
         toast="거래 3회 — 표본이 이만큼이면 수익률은 아직 아무 말도 하지 않는다"),
]


def screen_today():
    rows = ""
    for s in STRATS:
        rows += f"""
            <tr data-toast="{s['toast']}">
                <td><b>{s['name']}</b> <span class="k-dim">{s['ver']}</span></td>
                <td class="k-dim" style="font-size:11px"
                    data-toast="가상 보유 — 규칙이 시켰을 매수·매도를 리플레이한 결과. 실제 주문은 없다 (결정 01kzdxng)">{s['hold']}</td>
                <td class="k-num {s['rcls']}">{s['ret']}</td>
                <td class="k-num k-dim" data-toast="표본 — 3번 거래하고 +20%는 성적이 아니다">{s['n']}</td>
                <td class="k-num {s['bmcls']}" data-toast="같은 기간 KOSPI — 시장이 오른 것과 구분">{s['bm']}</td>
                <td class="k-dim" style="font-size:11px">{s['when']}</td>
                <td>{s['badge']}</td>
            </tr>"""

    inner = f"""
    <div class="k-col" style="flex:1">
        <div class="k-pane k-pane--lead" style="flex:1">
            <div class="k-pane-title"
                 data-toast="행 하나 = 등록 전략 하나의 현재 상황. 미등록 연구 스펙은 여기 없다 — 행이 생기는 순간이 곧 등록 확정 (결정 01kzdyrv)">내 전략들에 무슨 일이 있나
                <span class="k-badge">PN-STRAT</span>
                <span class="k-badge" style="margin-left:auto"
                      data-toast="일 단위 사후 리플레이 — 장중 실시간 손익은 v0에 없다. 판정에 필요한 것은 종가 기준 성적이다">종가 기준 · 08-07 계산</span></div>
            <div class="k-ans"
                 data-toast="답 스트립은 상태의 결정론적 렌더링이다 (결정 01kzdyrv) — 카운트를 문장 틀에 끼운다. 우선순위: 판정 > 오늘 종료 > 관찰 중 > 0건. LLM은 UI 문구를 쓰지 않는다">
                <span class="k-stamp k-stamp--sm">노이즈</span>
                <span class="k-ans-word">판정 1건 도착 · 관찰 2건 진행 중</span>
                <span class="k-ans-sub">gap-open 종료 07-30 · 행을 누르면 리포트가 열립니다</span>
            </div>
            <div class="k-pane-body" style="overflow:auto">
                <table class="k-table k-table--fit">
                    <tr><th>전략</th><th>가상 보유</th><th class="k-num">수익률</th>
                        <th class="k-num">거래</th><th class="k-num">벤치마크</th><th>남은 날</th><th>판정</th></tr>
                    {rows}
                </table>
                <div class="k-notice" style="margin:12px"
                     data-toast="무집행 추적 — 비용은 모델에 있지만 그 가격에 실제로 살 수 있었는지는 모른다">
                    무집행 추적입니다. 거래세·수수료·호가단위는 반영되지만
                    <b>체결 가능성은 반영되지 않습니다</b>.
                </div>
            </div>
        </div>
        <div style="display:flex;gap:8px;height:250px;flex:none">
            <div class="k-pane" style="flex:1.2">
                <div class="k-pane-title">오늘 볼 종목 <span class="k-badge k-badge--amber">고정 목록</span>
                    <span class="k-badge k-badge--live" style="margin-left:auto">● 5/41</span></div>
                <div class="k-pane-body" style="overflow:auto">
                    <table class="k-table">
                        <tr><th>종목</th><th class="k-num">현재가</th><th class="k-num">등락</th></tr>
                        <tr data-toast="행을 누르면 고정 안 된 pane이 이 종목으로 바뀐다"><td>삼성전자</td><td class="k-num">87,300</td><td class="k-num k-up">▲ +1.04%</td></tr>
                        <tr data-toast="행을 누르면 고정 안 된 pane이 이 종목으로 바뀐다"><td>SK하이닉스</td><td class="k-num">241,500</td><td class="k-num k-up">▲ +2.31%</td></tr>
                        <tr data-toast="행을 누르면 고정 안 된 pane이 이 종목으로 바뀐다"><td>NAVER</td><td class="k-num">173,400</td><td class="k-num k-up">▲ +0.35%</td></tr>
                    </table>
                </div>
            </div>
            <div class="k-pane" style="flex:1">
                <div class="k-pane-title">누가 사고 파나 <span class="k-badge">005930 · 일별</span></div>
                <div class="k-pane-body" style="padding:10px 12px">
                    <div class="mk4-flowline" data-toast="보조 pane의 답 스트립은 축소형 — 리드와의 위계 차이를 스트립 크기로도 만든다">
                        외국인 <b class="k-num k-up">+1,240억</b> · 기관 <b class="k-num k-down">-742억</b></div>
                    <div class="k-dim" style="font-size:10.5px;margin-top:8px">
                        2026-08-06 종가 기준 · 수급 축적 D+1</div>
                </div>
            </div>
            <div class="k-pane" style="flex:1">
                <div class="k-pane-title">무슨 일이 났나 <span class="k-badge">DART</span></div>
                <div class="k-pane-body" style="overflow:auto">
                    <table class="k-table">
                        <tr data-toast="원문은 외부 브라우저로 연다"><td>삼성전자<br><span class="k-dim" style="font-size:10px">단일판매·공급계약 체결</span></td><td class="k-num k-dim" style="font-size:10px">08:41</td></tr>
                        <tr data-toast="원문은 외부 브라우저로 연다"><td>SK하이닉스<br><span class="k-dim" style="font-size:10px">자기주식 취득 결정</span></td><td class="k-num k-dim" style="font-size:10px">어제</td></tr>
                    </table>
                </div>
            </div>
        </div>
    </div>"""
    cmd = cmd_global(
        '<span class="k-cmd-token">전략</span><span class="k-cmd-caret"></span>',
        hint="터미널이 필요하면 리서치 탭 — 또는 '터미널'을 치면 여기에도 놓입니다")
    return frame(
        "today", "오늘 — 등록 전략의 소비 화면", "SH-TODAY §6 · 결정 01kzdxn9·01kzdyrv",
        app(inner, cmd, active="오늘"),
        "아침 몇 분의 점검 화면. 전략 목록이 리드이고, 답 스트립의 문장은 <b>앱이 만든다</b> — "
        "상태 카운트를 유한한 틀에 끼우는 결정론 템플릿이고, LLM이 쓴 텍스트는 터미널 밖에 "
        "나오지 않는다. 행 하나 = 등록 전략 하나의 현재 상황(가상 보유·수익률·표본·벤치마크·남은 "
        "날·판정)이며 갱신은 일 단위 사후 리플레이다 — 이 화면의 질문은 '지금 얼마 벌었나'가 "
        "아니라 '내 전략이 진짜인가'이기 때문이다. 미등록 스펙은 이 목록에 없다: 여기 행이 "
        "생기는 것 자체가 등록 확정이고, 그것이 리서치와 오늘을 가르는 경계다.")


# ─────────────────────────────────────────────────────────────────

MK4_CSS = """
/* ── 구성안 v4 전용 (mk4-) ── */
.mk4-idx { font-size: 11px; color: var(--k-ink-2); display: flex; gap: 4px; align-items: baseline; }
.mk4-idx b { font-size: 11.5px; }
.mk4-fullhead { display: flex; align-items: center; gap: 18px; padding: 14px 28px; background: var(--k-bg-2); border-bottom: 1px solid var(--k-line-strong); }
.mk4-step { font-size: 12px; color: var(--k-ink-2); }
.mk4-wsteps { margin-left: auto; display: flex; gap: 22px; }
.mk4-wstep { font-size: 11.5px; color: var(--k-ink-3); display: flex; gap: 7px; align-items: center; }
.mk4-wstep b { font: 700 10px var(--k-mono); border: 1px solid var(--k-line-strong); padding: 2px 7px; }
.mk4-wstep.on { color: var(--k-ink); }
.mk4-wstep.on b { color: var(--k-amber); border-color: var(--k-amber-dim); background: var(--k-amber-bg); }
.mk4-wstep.done { color: var(--k-ink-3); }
.mk4-wstep.done b { color: var(--k-amber); border-color: var(--k-line); }
.mk4-fullbody { flex: 1; display: flex; gap: 28px; padding: 26px 28px; min-height: 0; overflow: hidden; }
.mk4-fullmain { flex: 1; min-width: 0; }
.mk4-fullside { width: 300px; flex: none; display: flex; flex-direction: column; gap: 14px; }
.mk4-fullfoot { display: flex; align-items: center; justify-content: space-between; padding: 13px 28px; background: var(--k-bg-2); border-top: 1px solid var(--k-line-strong); }
.mk4-sidebox { border: 1px solid var(--k-line); background: var(--k-bg-1); padding: 12px 14px; }
.mk4-siderow { font-size: 11.5px; color: var(--k-ink-2); padding: 6px 0; border-bottom: 1px solid var(--k-line); cursor: pointer; }
.mk4-siderow:last-child { border-bottom: none; }
.mk4-siderow:hover { color: var(--k-ink); }
.mk4-sect { font: 700 10.5px var(--k-sans); color: var(--k-ink-3); letter-spacing: .04em; margin-top: 16px; border-bottom: 1px solid var(--k-line); padding-bottom: 6px; }
.mk4-colrow { display: flex; align-items: center; gap: 14px; padding: 13px 4px; border-bottom: 1px solid var(--k-line); font-size: 12.5px; }
.mk4-colrow:hover { background: var(--k-bg-2); }
.mk4-colname { width: 150px; flex: none; display: flex; gap: 7px; align-items: center; font-weight: 700; }
.mk4-coldata { flex: 1; color: var(--k-ink-2); font-size: 11.5px; line-height: 1.5; }
.mk4-colkey { flex: none; display: flex; gap: 6px; align-items: center; }
.mk4-key { border: 1px solid var(--k-line-strong); background: var(--k-bg-0); padding: 6px 10px; font-size: 11px; color: var(--k-ink-2); }
.mk4-plugin { border-style: dashed; border-color: var(--k-line); }
.mk4-trust { border: 1px solid var(--k-amber-dim); background: var(--k-amber-bg); padding: 12px 14px; font-size: 11.5px; line-height: 1.7; color: var(--k-ink-2); }
.mk4-trust b { color: var(--k-ink); }
.mk4-banner { display: flex; align-items: center; gap: 8px; padding: 9px 16px; font-size: 12px; color: var(--k-ink); background: var(--k-amber-bg); border-bottom: 1px solid var(--k-amber-dim); }
.mk4-progrow { display: flex; align-items: center; gap: 8px; font-size: 11.5px; margin-bottom: 10px; }
.mk4-progrow > span:first-child { color: var(--k-ink-2); flex: none; }
.mk4-progrow > span:last-child { width: 40px; text-align: right; flex: none; }
.mk4-prog { flex: 1; height: 8px; background: var(--k-bg-0); border: 1px solid var(--k-line); }
.mk4-prog i { display: block; height: 100%; background: var(--k-amber-dim); }
.mk4-hz { display: inline-flex; flex-direction: column; margin-right: 26px; }
.mk4-hz i { font: 9.5px var(--k-sans); color: var(--k-ink-3); font-style: normal; }
.mk4-hz b { font-size: 17px; }
.mk4-actbar { display: flex; align-items: center; justify-content: space-between; padding: 9px 14px; border-top: 1px solid var(--k-line); background: var(--k-bg-2); }
.mk4-run { color: var(--k-amber); border-color: var(--k-amber-dim); }
.mk4-flowline { font-size: 12px; color: var(--k-ink-2); }
"""

PRESET_NOTES = """
    { screen: "onboard1", x: 84, y: 30, text: "keychain 안내는 우측 레일의 고정 요소다(결정 01kzdz3s) — 키를 건네는 바로 그 순간이 local-first 신뢰 명제를 말할 자리다. 값은 화면·로그 어디에도 안 나온다." },
    { screen: "onboard1", x: 40, y: 40, text: "이 표는 고정이 아니라 설치된 수집기 레지스트리의 렌더링이다(결정 01kzdr99). KRX처럼 id/pw가 필요한 채널은 공식 지원하지 않는다(결정 01kzdr2d) — 그 몫은 플러그인 생태계다." },
    { screen: "onboard1", x: 84, y: 74, text: "핀 #1 반영(결정 01kze1r7) — 다이얼로그가 아니라 전체 화면이고, 본문 표면은 설정 > 수집기 섹션과 같은 컴포넌트다. 온보딩 전용 UI를 따로 만들지 않는다." },
    { screen: "onboard2", x: 50, y: 40, text: "재배포 가능한 것만 번들, 차별 요소일수록 개인 수집 — 수급 이력이 그 예다. 온보딩은 수집 '시작'까지만 책임지고 완료는 수집 상태 pane이 이어받는다." },
    { screen: "landing", x: 50, y: 14, text: "온보딩 3/3은 화면이 아니라 게이트다. 완료 조건 = 에이전트의 첫 trdr CLI 호출 감지 — BYO라도 이 순간은 검증 가능하다. 배너 소멸이 곧 온보딩 완료." },
    { screen: "landing", x: 24, y: 50, text: "빈 pane을 미리 깔지 않는다(결정 01kzdwff) — 빈 차트가 있으면 '화면이 뭘 말하는지 모르겠다'를 첫 화면에서 재현한다. 지금 있는 두 pane은 둘 다 자기 질문에 답하고 있다." },
    { screen: "landing", x: 50, y: 96, text: "입력 표면 2개의 분업(결정 01kzdy9k): 명령줄은 앱에게, 터미널은 에이전트에게. 위치가 곧 수신자다. 명령줄에 자연어를 치면 REFUSE + 터미널 안내." },
    { screen: "growth", x: 30, y: 30, text: "작업이 pane을 놓는다 — 에이전트가 스펙을 저장하자 스펙 pane이 나타났다. PLACE 모델을 설명 없이 실제 동작으로 가르치는 첫 경험." },
    { screen: "research", x: 30, y: 72, text: "백테스트의 답 스트립 = 리플레이 지평 뷰(결정 01kzdxng). '그때 등록했다면' 시점을 잡고 1일·7일·1개월·1년을 본다. 탐색은 자유, 근거는 등록 후 out-of-sample뿐." },
    { screen: "research", x: 42, y: 90, text: "[이 스펙으로 등록]이 생산→소비 다리다(§7). 실험을 어느 정도 해본 뒤의 다음 행동이 결과를 보는 바로 그 pane의 액션 층에 있다." },
    { screen: "research", x: 82, y: 30, text: "터미널이 리드다 — 이 화면의 질문 '다음 가설을 어떻게 만드나'의 답은 에이전트의 지금 작업이다. 대화 UI를 따로 만들지 않는다: xterm 안의 에이전트 CLI TUI가 곧 대화 화면(BYO)." },
    { screen: "today", x: 50, y: 20, text: "답 스트립 문장은 앱이 만든다(결정 01kzdyrv) — 카운트를 유한한 틀에 끼우는 결정론 템플릿. 우선순위: 판정 > 오늘 종료 > 관찰 중 > 0건. 성적 표면에 LLM 윤색이 들어오면 정직성 규율과 모순이다." },
    { screen: "today", x: 50, y: 44, text: "행 하나 = 등록 전략 하나. 미등록 스펙은 없다 — 행이 생기는 순간이 등록 확정이고, 이것이 리서치(수정 가능)와 오늘(동결)을 가르는 경계다(결정 01kzdxn9)." },
    { screen: "today", x: 74, y: 34, text: "갱신은 일 단위 사후 리플레이 — 장중 실시간 손익은 v0에 없다. 이 화면의 질문은 '지금 얼마 벌었나'가 아니라 '내 전략이 진짜인가'다." }
"""


def check_balance(html):
    for tag in ("div", "section", "table", "tr", "td", "span", "button"):
        opens = len(re.findall(rf"<{tag}[\s>]", html))
        closes = html.count(f"</{tag}>")
        assert opens == closes, f"<{tag}> imbalance: {opens} open / {closes} close"


def main():
    html = TEMPLATE.read_text(encoding="utf-8")
    html = html.replace("<title>주제 이름 — 플로우 목업</title>", f"<title>{TITLE}</title>")
    html = html.replace('<div class="deck-title">주제 이름', f'<div class="deck-title">{TITLE.split(" — ")[0]}')

    kit_block = ("/* ═══ [KIT] trdr ui-kit 인라인 (정본: artifacts/ui-kit/kit.css) ═══ */\n"
                 + KIT_CSS + MK_CSS + MK4_CSS + "\n/* ═══ /[KIT] ═══ */")
    html = re.sub(r"/\* ═══ \[KIT\].*?/\[KIT\] ═══ \*/", lambda m: kit_block, html, flags=re.S)

    screens = "\n".join([
        screen_onboard1(),
        arrow("필수 키 검증 통과"),
        screen_onboard2(),
        arrow("수집 시작하고 진입"),
        screen_landing(),
        arrow("에이전트 첫 CLI 호출 감지 — 배너 소멸"),
        screen_growth(),
        arrow("탐색 백테스트 도착 — pane이 더 놓인다"),
        screen_research(),
        arrow("등록 확정 — 오늘 탭에 행 생성"),
        screen_today(),
    ])
    check_balance(screens)
    screens_block = "<!-- ═══ [SCREENS] ═══ -->\n" + screens + "\n<!-- ═══ /[SCREENS] ═══ -->"
    html = re.sub(r"<!-- ═══ \[SCREENS\].*?/\[SCREENS\] ═══ -->", lambda m: screens_block, html, flags=re.S)

    html = re.sub(r"var PRESET_NOTES = \[.*?\];",
                  lambda m: "var PRESET_NOTES = [" + PRESET_NOTES + "];", html, flags=re.S)
    html = html.replace('var MOCKUP_VERSION = "v1";', f'var MOCKUP_VERSION = "{VERSION}";')
    html = html.replace(
        "<li><b>v1</b> 최초 작성</li>",
        "<li><b>v4</b> 최초 작성 — 화면 구성안(SCREEN_COMPOSITION.md) 6프레임: "
        "온보딩 2장 + 착지 게이트 + 성장 + 리서치 완성형 + 오늘. 구조는 문서가 정본.</li>"
        "<li><b>v5</b> #1 반영 — 온보딩 1/3을 다이얼로그에서 전체 화면으로 재구성. "
        "수집기 표면을 설정 > 수집기와 같은 컴포넌트로 분리(결정 01kze1r7), 우측 레일에 "
        "keychain 안내·키 발급 안내·재사용 안내, 상단에 3단계 진행 표시. 2/3도 같은 셸로 통일.</li>",
    )

    OUT.write_text(html, encoding="utf-8")
    print(f"wrote {OUT} ({len(html):,} bytes)")


if __name__ == "__main__":
    main()
