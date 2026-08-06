#!/usr/bin/env python3
"""trdr "내 화면 만들기" 목업 조립기 (목업 v3 — 파일 3).

docs/USER_JOURNEY.md P1(S5~S8)과 docs/SCREEN_ACTIONS.md의 액션표에서
기계적으로 도출한 6프레임. 결정 01kzbjzf(전부 pane이다) · 01kzbq7y(성적은
나중에 계산) · 01kzbqh6(성적의 정의) · 01kzbqr7(성적 3열)을 화면으로 옮긴다.

에이전트 프레임 2개(S8)는 w-b2v8x가 끝난 뒤 v3.1로 붙인다.
공용 셸·SVG는 assemble_flow에서 가져온다.
"""
import pathlib
import re

from assemble_flow import KIT_CSS, MK_CSS, TEMPLATE, candles_svg, cmdline

ROOT = pathlib.Path(__file__).resolve().parent.parent
OUT = ROOT / "2026-08-06_trdr-compose-flow.html"
TITLE = "trdr 화면 목업 — 내 화면 만들기"
VERSION = "v3"


# ─────────────────────────────────────────────────────────────────
# 공용 — 이 파일의 셸 (프리셋이 탭이다. 검증 기록은 pane이라 탭이 아니다)
# ─────────────────────────────────────────────────────────────────

def topbar(active="리서치", saved=False):
    tabs = ""
    for name, toast in (("오늘", "프리셋 '내 가설의 오늘' — 전략 목록·종목 목록·수급·공시"),
                        ("리서치", "프리셋 '리서치' — 스펙·차트·백테스트·터미널")):
        on = " on" if name == active else ""
        tabs += f'<button class="k-toptab{on}" data-toast="{toast}">{name}</button>'
    if saved:
        on = " on" if active == "내 배치 1" else ""
        tabs += (f'<button class="k-toptab{on}" data-toast="A-G-11로 저장한 내 배치 — '
                 f'이름 변경·삭제는 설정에서">내 배치 1</button>')
    tabs += ('<button class="k-toptab" style="color:var(--k-ink-3)" '
             'data-toast="A-G-11 — 지금 배치를 이름 붙여 저장">＋</button>')
    return f"""
    <div class="k-topbar">
        <span class="k-logo">trdr</span>
        <nav class="k-topbar-tabs">{tabs}</nav>
        <div class="k-topbar-right">
            <span class="k-badge k-badge--live" data-toast="KIS 웹소켓 — 화면에 보이는 종목이 슬롯을 쓴다">● KIS WS</span>
            <span class="k-clock">09:04:11 KST</span>
        </div>
    </div>"""


def app(inner, cmd, extra="", active="리서치", saved=False):
    return f'<div class="k-app mk-desk">{topbar(active, saved)}{inner}{cmd}{extra}</div>'


def frame(sid, title, small, device, note):
    return f"""
    <section class="flow-step" data-screen="{sid}" data-title="{title}">
        <div class="step-head">{title} <small>{small}</small></div>
        <div class="device mk-device">{device}</div>
        <div class="step-note">{note}</div>
    </section>"""


def arrow(label):
    return f'<div class="flow-arrow" data-label="{label}"></div>'


def term_pane(lines, height=150):
    body = "".join(lines)
    return f"""
        <div class="k-pane mk-term-pane" style="height:{height}px">
            <div class="k-pane-title">터미널 <span class="k-badge">PN-TERM · 리서치 프리셋에 포함</span></div>
            <div class="k-term">{body}</div>
        </div>"""


# ─────────────────────────────────────────────────────────────────
# 1. 첫 진입 — 전부 빈 화면 (US-19 · A-G-03)
# ─────────────────────────────────────────────────────────────────

PANE_CATALOG = [
    ("차트", "ch", "종목 차트", "티커만 쳐도 열립니다"),
    ("목록", "list", "종목 목록", "고정 목록 · 전략 유니버스 · 관찰 중"),
    ("전략", "strat", "전략 목록", "등록한 전략의 지금 성적"),
    ("수급", "flow", "투자자별 순매수", "1999년부터 11분류"),
    ("공시", "disc", "공시 피드", "DART"),
    ("스펙", "spec", "전략 스펙", "파일이 정본입니다"),
    ("관찰", "obs", "한 등록의 상세", "전략 목록에서도 열립니다"),
    ("수집", "data", "수집 상태", "소스별 최신 일자"),
    ("터미널", "term", "에이전트 자리", "이미 아래에 있습니다"),
]


def screen_empty():
    rows = ""
    for i, (mn, alias, desc, sub) in enumerate(PANE_CATALOG):
        sel = " sel" if i == 0 else ""
        rows += f"""
            <div class="k-palette-row{sel}" data-toast="{mn} — {desc}. 명령줄이 곧 pane 카탈로그다 (결정 01kzbjzf)">
                <span class="k-palette-mn">{mn} <span class="k-dim">{alias}</span></span>
                <span class="k-palette-desc">{desc}</span>
                <span class="k-palette-sub">{sub}</span>
            </div>"""

    palette = f"""
        <div class="mk-palette-wrap" style="right:24px;bottom:186px">
        <div class="k-palette">
            <div class="k-palette-row" style="border-bottom:1px solid var(--k-line-strong);cursor:default">
                <span class="k-palette-mn k-amber-t">놓을 수 있는 pane</span>
                <span class="k-palette-desc">티커를 치면 차트가, 명령을 치면 그 pane이 놓입니다</span>
                <span class="k-palette-sub">Tab — 에이전트 모드</span>
            </div>
            {rows}
        </div>
        </div>"""

    inner = f"""
    <div class="mk-main">
        <div class="k-pane mk-center" style="border-left:none">
            <div class="k-pane-body mk-empty">
                <div class="mk-empty-box">
                    <div class="mk-empty-t">화면이 비어 있습니다</div>
                    <div class="mk-empty-d">
                        보고 싶은 것을 명령줄에 치면 그 자리에 pane이 놓입니다.<br>
                        데이터는 준비됐습니다 — 일봉 1,173종목 · 수급 1999~ · 공시.
                    </div>
                    <div class="mk-empty-ex">
                        <span data-toast="A-G-02 — 티커만 = 차트 pane">005930</span>
                        <span data-toast="A-G-02 — 티커 + 명령">005930 수급</span>
                        <span data-toast="A-G-02 — 명령만 = 종목 없는 pane">목록</span>
                    </div>
                    <div class="k-dim" style="font-size:11px;margin-top:14px">
                        전략이 0개, 등록이 0건입니다. 첫 가설은 아래 터미널에서
                        에이전트와 만들 수 있습니다.
                    </div>
                </div>
            </div>
        </div>
    </div>
    {term_pane([
        '<div><span class="t-prompt">❯</span> <span class="k-cmd-caret"></span></div>',
        '<div class="t-dim">내 에이전트를 여기서 실행합니다 (claude · codex …). '
        'trdr CLI가 PATH에 있고, 작업 폴더가 이 창의 작업 디렉토리입니다.</div>',
    ], height=140)}"""

    cmd = cmdline(
        '<span class="k-dim">005930 · 삼성전자 · 목록 · 전략 …</span>',
        hint="명령줄을 누르면 놓을 수 있는 pane이 전부 보입니다")
    return frame(
        "empty", "첫 진입 — 전부 빈 화면",
        "US-19 · A-G-03 · S5",
        app(inner, cmd, palette),
        "빈 상태가 곧 카탈로그다. 카탈로그 메뉴를 따로 만들지 않기로 했으므로(결정 01kzbjzf) "
        "명령줄이 스스로 무엇을 할 수 있는지 말해야 한다 — 이것이 미결 1의 답 후보다. "
        "첫 진입 프리셋이 '리서치'인 이유는 등록 0건이면 '오늘'이 전부 비어 있고, "
        "첫 행동에 필요한 터미널이 리서치에만 있기 때문이다.")


# ─────────────────────────────────────────────────────────────────
# 2. 명령줄로 pane 놓기 (US-20 · A-G-02)
# ─────────────────────────────────────────────────────────────────

def mini_chart():
    return candles_svg(w=880, h=430, seed=4)


def flow_bars():
    rows = ""
    data = [("외국인", 62, "k-up", "+1,240억"), ("기관", -38, "k-down", "-742억"),
            ("개인", -24, "k-down", "-498억"), ("연기금", 18, "k-up", "+361억"),
            ("금융투자", -12, "k-down", "-233억")]
    for name, pct, cls, val in data:
        w = abs(pct)
        side = "left:50%" if pct > 0 else f"right:50%"
        rows += f"""
            <div class="mk-flowrow">
                <span>{name}</span>
                <div class="mk-flowbar"><i class="{cls}" style="{side};width:{w * 0.5}%"></i></div>
                <span class="k-num {cls}">{val}</span>
            </div>"""
    return rows


def screen_place():
    inner = f"""
    <div class="mk-main">
        <div class="k-pane mk-center" style="border-left:none">
            <div class="k-pane-title">
                005930 삼성전자 · 일봉
                <span class="k-num" style="font-size:12px;color:var(--k-ink)">87,300</span>
                <span class="k-num k-up" style="font-size:11px">▲ +1.04%</span>
                <span style="margin-left:auto;display:flex;gap:6px">
                    <span class="k-badge k-badge--live" data-toast="화면에 보이는 pane이 실시간 슬롯을 쓴다">● 실시간</span>
                    <span class="k-badge mk-pin" data-toast="A-G-08 — 고정하면 다음 티커 명령이 이 pane을 바꾸지 않는다">◇ 고정</span>
                </span>
            </div>
            <div class="k-pane-body">{mini_chart()}</div>
        </div>
        <div class="k-pane" style="width:360px;flex:none;border-top:none;border-right:none">
            <div class="k-pane-title">
                005930 수급 <span class="k-badge">일별 · 11분류</span>
                <span style="margin-left:auto"><span class="k-badge mk-pin" data-toast="A-G-08 — 고정 안 됨">◇ 고정</span></span>
            </div>
            <div class="k-pane-body" style="padding:12px 14px">
                <div class="k-dim" style="font-size:11px;margin-bottom:10px">2026-08-05 · 순매수 (억원)</div>
                {flow_bars()}
                <hr class="k-hr" style="margin:14px 0 10px">
                <div class="k-dim" style="font-size:10.5px" data-toast="A-FLW-04 — 어느 소스에서 언제 받은 값인지">
                    출처 KRX 인증 루트 · 최신 2026-08-05 · 1999-01-04부터 보유
                </div>
            </div>
        </div>
    </div>
    {term_pane([
        '<div><span class="t-prompt">❯</span> <span class="k-cmd-caret"></span></div>',
    ], height=110)}"""

    cmd = cmdline(
        '<span class="k-cmd-token">005930</span> <span class="k-cmd-token">수급</span>'
        '<span class="k-cmd-caret"></span>',
        hint="같은 종류 pane이 있으면 내용만 바뀝니다 — 고정한 pane은 예외")
    return frame(
        "place", "명령줄로 pane 놓기",
        "US-20 · A-G-02 · §3.2",
        app(inner, cmd),
        "<b>005930</b>이 차트를 놓고, <b>005930 수급</b>이 수급 pane을 옆에 놓았다. "
        "관심종목을 '등록'하는 별도 개체가 없다 — 보는 것을 늘리는 행위가 곧 pane을 놓는 것이다. "
        "같은 명령을 다른 티커로 다시 치면 고정하지 않은 pane의 내용이 바뀐다(제안 01kzbjzs). "
        "에이전트도 CLI로 같은 함수를 부른다.")


# ─────────────────────────────────────────────────────────────────
# 3. 목록 pane과 출처 (US-21 · A-LIST-01)
# ─────────────────────────────────────────────────────────────────

PINNED = [("삼성전자", "005930", "87,300", "▲ +1.04%", "k-up"),
          ("SK하이닉스", "000660", "241,500", "▲ +2.31%", "k-up"),
          ("현대차", "005380", "198,000", "▼ -0.72%", "k-down"),
          ("NAVER", "035420", "173,400", "▲ +0.35%", "k-up"),
          ("카카오", "035720", "41,250", "▼ -1.18%", "k-down")]

UNIVERSE = [("LG에너지솔루션", "373220", "momentum_20d 1위"),
            ("삼성전자", "005930", "momentum_20d 2위"),
            ("SK하이닉스", "000660", "momentum_20d 3위"),
            ("한화에어로스페이스", "012450", "순위 밖 — 참고"),
            ("두산에너빌리티", "034020", "순위 밖 — 참고")]

HOLDING = [("SK하이닉스", "000660", "08-01 진입", "+4.2%", "k-up"),
           ("LG에너지솔루션", "373220", "08-04 진입", "-1.6%", "k-down"),
           ("삼성전자", "005930", "07-28 진입", "+2.9%", "k-up")]


def list_pinned():
    rows = ""
    for name, code, price, chg, cls in PINNED:
        rows += (f'<tr data-toast="A-LIST-02 — 고정 안 된 차트·수급·공시 pane이 이 종목으로 바뀐다">'
                 f'<td><span class="mk-star" data-toast="A-LIST-03 — 고정 목록에서 빼기">★</span> {name}'
                 f'<br><span class="k-dim" style="font:10px var(--k-mono)">{code}</span></td>'
                 f'<td class="k-num">{price}</td><td class="k-num {cls}">{chg}</td></tr>')
    return f"""
        <div class="k-pane mk-lp" style="border-left:none">
            <div class="k-pane-title">종목 목록
                <span class="k-badge k-badge--amber">고정 목록</span>
                <span class="k-badge k-badge--live" style="margin-left:auto" data-toast="A-LIST-05 — 이 목록이 쓰는 실시간 슬롯. 상한은 41">● 5/41</span>
            </div>
            <div class="mk-srcbar">
                <span class="on" data-toast="사람이 넣어둔 종목 — 순서까지 저장된다">고정 목록</span>
                <span data-toast="스펙에서 도출 — 매일 바뀔 수 있다">전략 유니버스</span>
                <span data-toast="관찰 중 전략이 지금 들고 있는 종목">관찰 중</span>
            </div>
            <div class="k-pane-body" style="overflow:auto">
                <table class="k-table">
                    <tr><th>종목</th><th class="k-num">현재가</th><th class="k-num">등락</th></tr>
                    {rows}
                </table>
            </div>
        </div>"""


def list_universe():
    rows = ""
    for name, code, why in UNIVERSE:
        rows += (f'<tr data-toast="스펙이 뽑은 종목 — ★로 고정 목록에 넣을 수 있다">'
                 f'<td>{name}<br><span class="k-dim" style="font:10px var(--k-mono)">{code}</span></td>'
                 f'<td class="k-dim" style="font-size:10.5px">{why}</td></tr>')
    return f"""
        <div class="k-pane mk-lp">
            <div class="k-pane-title">종목 목록
                <span class="k-badge">전략 유니버스</span>
                <span class="k-badge" style="margin-left:auto" data-toast="이 pane의 내용은 스펙이 정한다 — 사람이 못 고친다">momentum-3 v4</span>
            </div>
            <div class="mk-srcbar">
                <span data-toast="사람이 넣어둔 종목">고정 목록</span>
                <span class="on" data-toast="스펙에서 도출 — 매일 바뀔 수 있다">전략 유니버스</span>
                <span data-toast="관찰 중 전략이 지금 들고 있는 종목">관찰 중</span>
            </div>
            <div class="k-pane-body" style="overflow:auto">
                <table class="k-table">
                    <tr><th>종목</th><th>선정 이유</th></tr>
                    {rows}
                </table>
            </div>
        </div>"""


def list_holding():
    rows = ""
    for name, code, when, pl, cls in HOLDING:
        rows += (f'<tr data-toast="관찰 중 전략의 보유 — 무집행 추적이라 실제 주문은 없다">'
                 f'<td>{name}<br><span class="k-dim" style="font:10px var(--k-mono)">{code}</span></td>'
                 f'<td class="k-dim" style="font-size:10.5px">{when}</td>'
                 f'<td class="k-num {cls}">{pl}</td></tr>')
    return f"""
        <div class="k-pane mk-lp" style="border-right:none">
            <div class="k-pane-title">종목 목록
                <span class="k-badge">관찰 중</span>
                <span class="k-badge" style="margin-left:auto">momentum-3 · D-23</span>
            </div>
            <div class="mk-srcbar">
                <span data-toast="사람이 넣어둔 종목">고정 목록</span>
                <span data-toast="스펙에서 도출">전략 유니버스</span>
                <span class="on" data-toast="관찰 중 전략이 지금 들고 있는 종목">관찰 중</span>
            </div>
            <div class="k-pane-body" style="overflow:auto">
                <table class="k-table">
                    <tr><th>종목</th><th>진입</th><th class="k-num">참고</th></tr>
                    {rows}
                </table>
                <div class="k-notice" style="margin:10px" data-toast="A-STR-02 — 종료일 전 숫자는 전부 참고용">
                    이 손익은 <b>참고용</b>입니다. 결과는 종료일(D-0)에 한 번 납니다.
                </div>
            </div>
        </div>"""


def screen_lists():
    inner = f"""
    <div class="mk-main">{list_pinned()}{list_universe()}{list_holding()}</div>
    {term_pane([
        '<div><span class="t-prompt">❯</span> trdr 목록 --출처 고정</div>',
        '<div class="t-out">고정 목록 5종목 · 실시간 5/41</div>',
        '<div><span class="t-prompt">❯</span> <span class="k-cmd-caret"></span></div>',
    ], height=110)}"""
    cmd = cmdline('<span class="k-cmd-token">목록</span><span class="k-cmd-caret"></span>',
                  hint="같은 종류 pane을 여러 개 놓고 출처를 다르게 둘 수 있습니다")
    return frame(
        "lists", "목록 pane과 출처",
        "US-21 · A-LIST-01 · 결정 01kzbjzf",
        app(inner, cmd),
        "볼 종목의 출처가 셋이다 — 사람이 고른 고정 목록, 스펙이 도출한 유니버스, "
        "관찰 중 전략이 든 종목. <b>셋을 한 목록에 섞지 않는다.</b> 각각이 목록 pane 하나의 "
        "출처가 되고, 섞을지 나눌지는 앱의 규칙이 아니라 사용자가 pane을 몇 개 놓느냐가 된다. "
        "이 3개를 한 화면에 놓은 것이 여기 그림이고, 하나만 놓아도 된다.")


# ─────────────────────────────────────────────────────────────────
# 4. 실시간 슬롯 배분 (US-22 · A-LIST-04~06)
# ─────────────────────────────────────────────────────────────────

BIG_LIST = [
    ("삼성전자", "005930", "87,300", "▲ +1.04%", "k-up", "live"),
    ("SK하이닉스", "000660", "241,500", "▲ +2.31%", "k-up", "live"),
    ("현대차", "005380", "198,000", "▼ -0.72%", "k-down", "live"),
    ("NAVER", "035420", "173,400", "▲ +0.35%", "k-up", "live"),
    ("카카오", "035720", "41,250", "▼ -1.18%", "k-down", "live"),
    ("LG에너지솔루션", "373220", "402,000", "▲ +3.02%", "k-up", "live"),
    ("셀트리온", "068270", "186,700", "▼ -0.43%", "k-down", "live"),
    ("두산에너빌리티", "034020", "33,700", "▲ +0.89%", "k-up", "delay"),
    ("한화에어로스페이스", "012450", "612,000", "▼ -1.51%", "k-down", "delay"),
    ("STX중공업", "071970", "—", "상장폐지 2024-03-21", "", "dead"),
]


def screen_slots():
    rows = ""
    for name, code, price, chg, cls, kind in BIG_LIST:
        if kind == "live":
            badge = '<span class="k-badge k-badge--live" style="font-size:9px">●</span>'
            toast = "화면에 보이는 종목이라 실시간 슬롯을 쓴다"
        elif kind == "delay":
            badge = ('<span class="k-badge mk-delay" data-toast="A-LIST-04 — 스크롤 밖이라 '
                     '실시간을 쓰지 않는다. 옛 가격을 지금 가격으로 착각하지 않게 라벨을 단다">지연 15분</span>')
            toast = "지연 시세 — 스크롤해서 보이게 하면 실시간으로 바뀐다"
        else:
            badge = ('<span class="k-badge mk-dead" data-toast="상폐 종목 — 시세가 없고 이력만 있다. '
                     '백테스트에는 쓰이고 시세 pane에서는 값이 뜨지 않는다 (미결 2)">이력만</span>')
            toast = "상장폐지 — 시세 없음. 백테스트 데이터에는 남아 있다"
        cls_attr = f" class=\"{cls}\"" if cls else ' class="k-dim"'
        rows += (f'<tr data-toast="{toast}"><td>{name} {badge}'
                 f'<br><span class="k-dim" style="font:10px var(--k-mono)">{code}</span></td>'
                 f'<td class="k-num">{price}</td><td class="k-num"{cls_attr}>{chg}</td></tr>')

    inner = f"""
    <div class="mk-main">
        <div class="k-pane" style="flex:1;border-left:none;border-right:none">
            <div class="k-pane-title">종목 목록
                <span class="k-badge k-badge--amber">고정 목록 · 60종목</span>
                <span class="k-badge k-badge--live" style="margin-left:auto"
                      data-toast="A-LIST-05 — 화면에 보이는 종목이 41 슬롯을 쓴다. 나머지는 지연">● 실시간 41/41</span>
            </div>
            <div class="k-pane-body" style="overflow:auto">
                <table class="k-table">
                    <tr><th>종목</th><th class="k-num">현재가</th><th class="k-num">등락</th></tr>
                    {rows}
                </table>
                <div class="mk-fold" data-toast="A-LIST-06 — 스크롤하면 보이는 행으로 구독이 옮겨간다">
                    ⌄ 아래 50종목 — 스크롤하면 실시간으로 바뀝니다
                </div>
            </div>
        </div>
        <div class="k-pane" style="width:330px;flex:none;border-top:none;border-right:none">
            <div class="k-pane-title">실시간 슬롯 <span class="k-badge">KIS 웹소켓</span></div>
            <div class="k-pane-body" style="padding:14px">
                <div class="k-kv"><dt>세션 상한</dt><dd class="k-num">41 종목</dd></div>
                <div class="k-kv"><dt>지금 쓰는 중</dt><dd class="k-num k-amber-t">41</dd></div>
                <div class="k-kv"><dt>지연으로 내려감</dt><dd class="k-num">19</dd></div>
                <div class="k-kv"><dt>시세 없음(상폐)</dt><dd class="k-num">3</dd></div>
                <hr class="k-hr" style="margin:12px 0">
                <div style="font-size:11.5px;line-height:1.7;color:var(--k-ink-2)">
                    <b class="k-amber-t">화면에 보이는 것이 실시간을 씁니다.</b><br>
                    목록을 스크롤하거나 pane을 닫으면 슬롯이 옮겨갑니다.
                    보이지 않는 종목에 실시간을 쓸 이유가 없기 때문입니다.
                </div>
                <div class="k-notice" style="margin-top:12px"
                     data-toast="확인 12 — KIS가 잦은 구독 교체를 견디지 못하면 사용자가 ★로 고정하는 방식으로 후퇴한다">
                    구현 확인 필요 — 스크롤마다 구독을 갈아치우는 것을 KIS가 견디는지.
                </div>
            </div>
        </div>
    </div>
    {term_pane(['<div><span class="t-prompt">❯</span> <span class="k-cmd-caret"></span></div>'], height=100)}"""

    cmd = cmdline('<span class="k-dim">티커 · 명령 …</span>',
                  hint="지연·이력만 라벨은 숨기지 않습니다")
    return frame(
        "slots", "실시간 슬롯과 지연",
        "US-22 · A-LIST-04~06 · 결정 01kzbjzf",
        app(inner, cmd),
        "실시간 상한(~41종목)을 숨기지 않는다. 배분 규칙은 '보이는 것이 쓴다' 하나이고, "
        "그래서 사용자가 슬롯을 관리할 필요가 없다. 지연 종목은 라벨과 지연 시간을 달아 "
        "옛 가격을 지금 가격으로 읽지 않게 한다. 상폐 종목은 시세 자리가 비고 이력만 남는다 — "
        "상폐 포함이 데이터의 차별 요소이므로 목록에서 지우지 않는다.")


# ─────────────────────────────────────────────────────────────────
# 5. 배치·고정·프리셋 저장 (US-23 · A-G-06~08·11)
# ─────────────────────────────────────────────────────────────────

DISCLOSURES = [("삼성전자", "단일판매·공급계약 체결", "08-06 08:41"),
               ("SK하이닉스", "주요사항보고서 (자기주식 취득)", "08-05 16:22"),
               ("현대차", "임원·주요주주 특정증권등 소유상황보고서", "08-05 14:07")]


def disc_rows():
    out = ""
    for name, title, when in DISCLOSURES:
        out += (f'<tr data-toast="A-DSC-03 — 원문은 외부 브라우저로 연다">'
                f'<td>{name}<br><span class="k-dim" style="font-size:10px">{title}</span></td>'
                f'<td class="k-num k-dim" style="font-size:10px">{when}</td></tr>')
    return f'<table class="k-table">{out}</table>'


def pinned_rows():
    out = ""
    for name, code, price, chg, cls in PINNED:
        out += (f'<tr data-toast="A-LIST-02 — 고정 안 된 pane이 이 종목으로 바뀐다. '
                f'차트는 고정돼 있어 그대로 남는다">'
                f'<td><span class="mk-star">★</span> {name}</td>'
                f'<td class="k-num">{price}</td><td class="k-num {cls}">{chg}</td></tr>')
    return f'<table class="k-table"><tr><th>종목</th><th class="k-num">현재가</th><th class="k-num">등락</th></tr>{out}</table>'


def screen_layout():
    def pin_pane(title, badge, pinned, body, style=""):
        pin = ('<span class="k-badge mk-pin on" data-toast="A-G-08 — 고정됨. 티커 명령이 이 pane을 바꾸지 않는다">◆ 고정</span>'
               if pinned else
               '<span class="k-badge mk-pin" data-toast="A-G-08 — 고정 안 됨. 다음 티커 명령이 이 pane의 내용을 바꾼다">◇ 고정</span>')
        return f"""
        <div class="k-pane" style="{style}">
            <div class="k-pane-title">{title} <span class="k-badge">{badge}</span>
                <span style="margin-left:auto">{pin}</span></div>
            <div class="k-pane-body" style="padding:8px 10px;overflow:hidden">{body}</div>
        </div>"""

    grid = f"""
    <div class="mk-main" style="flex-direction:column">
        <div style="display:flex;flex:1;min-height:0">
            {pin_pane("005930 삼성전자 · 일봉", "PN-CHART", True,
                      candles_svg(w=900, h=300, seed=9),
                      "flex:1;border-left:none;border-right:none")}
            {pin_pane("종목 목록", "고정 목록", False, pinned_rows(),
                      "width:320px;flex:none;border-top:none;border-right:none")}
        </div>
        <div style="display:flex;height:230px;flex:none">
            {pin_pane("005930 수급", "일별 · 11분류", False, flow_bars(),
                      "flex:1;border-left:none;border-right:none;border-bottom:none")}
            {pin_pane("공시", "DART", False, disc_rows(),
                      "width:320px;flex:none;border-right:none;border-bottom:none")}
        </div>
    </div>"""

    modal = """
    <div class="mk-pop" data-toast="A-G-11 — 지금 배치를 이름 붙여 저장. 저장은 배치한 자리에서 일어난다 (미결 3)">
        <div class="k-pane" style="border-color:var(--k-line-strong)">
            <div class="k-pane-title">지금 배치 저장</div>
            <div style="padding:14px 16px;display:flex;flex-direction:column;gap:12px">
                <div style="font-size:11.5px;color:var(--k-ink-2);line-height:1.7">
                    지금 화면의 pane 구성·크기·고정 상태를 이름 붙여 저장합니다.
                    상단 탭에 추가되고 언제든 돌아올 수 있습니다.
                </div>
                <div class="mk-input"><span class="k-num">내 배치 1</span><span class="k-cmd-caret"></span></div>
                <div class="k-kv"><dt>담기는 pane</dt><dd class="k-num">4</dd></div>
                <div class="k-kv"><dt>고정된 pane</dt><dd class="k-num">1</dd></div>
                <div style="display:flex;gap:8px;justify-content:flex-end">
                    <button class="k-btn" data-toast="저장하지 않고 닫는다 — 배치는 그대로 남는다">취소</button>
                    <button class="k-btn k-btn--amber" data-goto="strat">저장</button>
                </div>
                <div class="k-dim" style="font-size:10.5px;line-height:1.6">
                    이름 변경·삭제는 설정에서 합니다.<br>기본 프리셋 2개는 지워지지 않습니다.
                </div>
            </div>
        </div>
    </div>"""

    cmd = cmdline('<span class="k-dim">티커 · 명령 …</span>',
                  hint="pane은 드래그로 옮깁니다 — 에이전트는 배치를 바꾸지 못합니다")
    return frame(
        "layout", "배치·고정·프리셋 저장",
        "US-23 · A-G-06~08·11 · 제안 01kzbjzs",
        app(grid, cmd, modal, saved=False),
        "고정(핀)이 이 화면의 핵심이다. 고정하지 않은 pane은 다음 티커 명령의 대상이 되어 "
        "내용이 바뀌고, 고정한 pane은 그대로 남는다 — 브라우저 탭 고정과 같은 규칙이다. "
        "덕분에 차트를 여러 개 쌓지 않고 하나를 갈아끼우며 볼 수 있다. "
        "에이전트는 pane을 놓고 내용을 바꿀 수 있지만 <b>배치를 흩뜨리거나 닫지 못한다</b> — "
        "배치는 사용자의 것이다.")


# ─────────────────────────────────────────────────────────────────
# 6. 전략 목록 (US-24 후속 · A-STR-* · 결정 01kzbq7y·01kzbqh6·01kzbqr7)
# ─────────────────────────────────────────────────────────────────

STRATS = [
    dict(name="gap-open", ver="v2", state="결과", badge="결과 — 노이즈",
         bcls="mk-st-done", ret="+1.8%", rcls="k-flat", n="41", bm="+3.4%", bcls2="k-up",
         when="종료 07-30", ref=False,
         toast="A-STR-04 — 끝난 등록. 행을 누르면 결과 리포트가 열린다"),
    dict(name="momentum-3", ver="v4", state="기간 중", badge="D-23",
         bcls="mk-st-run", ret="+6.2%", rcls="k-up", n="12", bm="+2.1%", bcls2="k-up",
         when="종료 2026-08-29", ref=True,
         toast="A-STR-04 — 진행 중. 행을 누르면 그 등록의 관찰 상세가 열린다"),
    dict(name="meanrev-krx", ver="v1", state="기간 중", badge="D-71",
         bcls="mk-st-run", ret="-0.4%", rcls="k-down", n="3", bm="+2.1%", bcls2="k-up",
         when="종료 2026-10-16", ref=True,
         toast="거래 3회 — 표본이 이만큼이면 수익률은 아직 아무 말도 하지 않는다"),
    dict(name="flow-divergence", ver="v1", state="탐색", badge="등록 전",
         bcls="", ret="—", rcls="k-dim", n="—", bm="—", bcls2="k-dim",
         when="스펙 초안", ref=False,
         toast="아직 등록 전 — 자유롭게 고칠 수 있고, 성적은 탐색 백테스트로만 본다"),
]


def screen_strat():
    rows = ""
    for s in STRATS:
        ref = ('<span class="k-badge mk-ref" data-toast="A-STR-02 — 종료일 전 숫자는 참고용. '
               '결과 스탬프는 종료일에만 나온다">참고용</span>' if s["ref"] else "")
        stamp = ('<span class="k-stamp k-stamp--sm" data-toast="A-STR-06 — 종료일이 지난 등록에만 스탬프가 붙는다">노이즈</span>'
                 if s["state"] == "결과" else "")
        rows += f"""
            <tr data-toast="{s['toast']}">
                <td><b>{s['name']}</b> <span class="k-dim">{s['ver']}</span>
                    <br><span class="k-dim" style="font-size:10px">{s['when']}</span></td>
                <td><span class="k-badge {s['bcls']}">{s['badge']}</span> {stamp}</td>
                <td class="k-num {s['rcls']}">{s['ret']} {ref}</td>
                <td class="k-num k-dim">{s['n']}</td>
                <td class="k-num {s['bcls2']}">{s['bm']}</td>
            </tr>"""

    inner = f"""
    <div class="mk-main">
        <div class="k-pane" style="flex:1;border-left:none;border-right:none">
            <div class="k-pane-title">전략 목록
                <span class="k-badge">PN-STRAT</span>
                <span class="k-badge" style="margin-left:auto"
                      data-toast="A-STR-07 — 데이터를 백필한 뒤 다시 계산할 수 있다">저장된 데이터로 계산 · 09:04</span>
            </div>
            <div class="k-pane-body" style="overflow:auto">
                <table class="k-table mk-sttable">
                    <tr>
                        <th>전략</th><th>상태</th>
                        <th class="k-num">수익률</th>
                        <th class="k-num" data-toast="A-STR-08 — 표본. 3번 거래하고 +20%는 성적이 아니다">거래</th>
                        <th class="k-num" data-toast="A-STR-09 — 같은 기간 KOSPI. 시장이 오른 것과 구분">벤치마크</th>
                    </tr>
                    {rows}
                </table>
                <div class="k-notice" style="margin:12px"
                     data-toast="A-STR-10 — 비용은 모델에 있지만 체결 가능성은 모른다">
                    무집행 추적입니다. 거래세·수수료·호가단위는 반영되지만
                    <b>그 가격에 실제로 살 수 있었는지는 반영되지 않습니다</b> —
                    유동성이 낮은 종목일수록 성적이 실제보다 좋게 나옵니다.
                </div>
            </div>
        </div>
        <div class="k-pane" style="width:340px;flex:none;border-top:none;border-right:none">
            <div class="k-pane-title">성적이 무엇인가</div>
            <div class="k-pane-body" style="padding:14px;font-size:11.5px;line-height:1.75;color:var(--k-ink-2)">
                전략은 여러 정보로 <b>매수·매도를 잡는 규칙</b>이고,
                성적은 그 규칙대로 매매했다면 나왔을 수익률입니다.
                <hr class="k-hr" style="margin:12px 0">
                <div class="k-kv"><dt>계산 방식</dt><dd>저장된 데이터 재생</dd></div>
                <div class="k-kv"><dt>탐색과의 차이</dt><dd>구간뿐</dd></div>
                <div class="k-kv"><dt>고정된 것</dt><dd>해시 · 종료일</dd></div>
                <hr class="k-hr" style="margin:12px 0">
                <div>
                    등록이 해시를 고정한 시점 이후의 데이터는 전부 out-of-sample입니다.
                    그래서 <b class="k-amber-t">관찰이 도는 것이 아니라</b>, 물어볼 때 계산합니다 —
                    앱을 몇 달 안 켜도 결손이 생기지 않고, 수집이 밀리면 백필로 복구합니다.
                </div>
                <div class="k-notice" style="margin-top:12px"
                     data-toast="종료일을 미리 고정하는 것이 정지 시점을 묶는다">
                    아무 때나 볼 수 있으니 좋아 보일 때 끊고 싶어집니다.
                    그래서 <b>중단 액션이 없습니다</b> — 등록은 선언한 종료일까지 갑니다.
                </div>
            </div>
        </div>
    </div>
    {term_pane([
        '<div><span class="t-prompt">❯</span> trdr 전략</div>',
        '<div class="t-out">4개 · 기간 중 2 · 결과 1 · 탐색 1</div>',
        '<div class="t-dim">  momentum-3 v4  D-23  +6.2% (참고용)  12거래  BM +2.1%</div>',
        '<div><span class="t-prompt">❯</span> <span class="k-cmd-caret"></span></div>',
    ], height=130)}"""

    cmd = cmdline('<span class="k-cmd-token">전략</span><span class="k-cmd-caret"></span>',
                  hint="관찰 pane은 한 등록을 파고들 때만 엽니다")
    return frame(
        "strat", "전략 목록 — 성적 3열",
        "A-STR-* · 결정 01kzbq7y·01kzbqh6·01kzbqr7",
        app(inner, cmd, active="오늘"),
        "관찰을 상시 띄워두는 pane이 사라지고 <b>훑는 목록</b>이 그 자리에 온다. "
        "카운트다운은 종료일 열이 되고, 관찰 pane은 한 등록의 상세로 내려간다. "
        "성적을 숫자 하나로 두지 않는 이유는 목록이 정독하는 화면이 아니기 때문이다 — "
        "행에서 빠진 것은 잊힌다. meanrev-krx가 그 예다: -0.4%보다 <b>거래 3회</b>가 "
        "더 중요한 정보인데, 수익률만 있으면 그것을 볼 방법이 없다.")


# ─────────────────────────────────────────────────────────────────

MK3_CSS = """
/* ── 구성 목업 전용 (mk-, 파일 3) ── */
.mk-empty { display: flex; align-items: flex-start; justify-content: center; padding: 54px 0 0; }
.mk-empty-box { max-width: 520px; text-align: center; padding: 0 20px; }
.mk-empty-t { font-size: 15px; font-weight: 700; color: var(--k-ink); margin-bottom: 10px; }
.mk-empty-d { font-size: 12px; line-height: 1.8; color: var(--k-ink-2); }
.mk-empty-ex { display: flex; gap: 8px; justify-content: center; margin-top: 16px; }
.mk-empty-ex span { font: 11.5px var(--k-mono); color: var(--k-amber); border: 1px solid var(--k-amber-dim); padding: 4px 10px; cursor: pointer; }
.mk-empty-ex span:hover { background: var(--k-amber-bg); }
.mk-lp { width: 33.333%; flex: none; border-top: none; }
.mk-srcbar { display: flex; border-bottom: 1px solid var(--k-line); }
.mk-srcbar span { flex: 1; text-align: center; padding: 5px 0; font-size: 10.5px; color: var(--k-ink-3); border-right: 1px solid var(--k-line); cursor: pointer; }
.mk-srcbar span:last-child { border-right: none; }
.mk-srcbar span.on { color: var(--k-amber); background: var(--k-amber-bg); }
.mk-star { color: var(--k-amber); font-size: 10px; cursor: pointer; }
.mk-pin { cursor: pointer; }
.mk-pin.on { color: var(--k-amber); border-color: var(--k-amber-dim); }
.mk-delay { color: var(--k-ink-3); }
.mk-dead { color: var(--k-ink-3); border-style: dashed; }
.mk-ref { color: var(--k-ink-3); font-size: 9px; margin-left: 4px; }
.mk-fold { padding: 10px 12px; font-size: 11px; color: var(--k-ink-3); border-top: 1px dashed var(--k-line); cursor: pointer; }
.mk-flowrow { display: flex; align-items: center; gap: 8px; margin-bottom: 8px; font-size: 11px; }
.mk-flowrow > span:first-child { width: 60px; color: var(--k-ink-2); flex: none; }
.mk-flowrow > span:last-child { width: 68px; text-align: right; flex: none; }
.mk-flowbar { flex: 1; height: 10px; background: var(--k-bg-0); border: 1px solid var(--k-line); position: relative; }
.mk-flowbar i { position: absolute; top: 0; bottom: 0; display: block; }
.mk-flowbar i.k-up { background: var(--k-up); }
.mk-flowbar i.k-down { background: var(--k-down); }
.mk-pop { position: absolute; left: 232px; top: 46px; width: 330px; }
.mk-input { border: 1px solid var(--k-amber-dim); background: var(--k-bg-0); padding: 8px 10px; font: 12px var(--k-mono); }
.mk-sttable td { vertical-align: top; }
.mk-st-run { color: var(--k-amber); border-color: var(--k-amber-dim); }
.mk-st-done { color: var(--k-ink-2); }
"""

PRESET_NOTES = """
    { screen: "empty", x: 50, y: 62, text: "미결 1의 답 후보 — 팔레트가 곧 카탈로그이자 help다. 코치마크·오버레이 튜토리얼은 쓰지 않는다: 한 번 보고 사라지면 다시 못 찾고, 빈 상태 자체가 안내면 오버레이가 필요 없다." },
    { screen: "empty", x: 16, y: 88, text: "첫 진입 프리셋을 리서치로 둔 이유 — 등록 0건이면 '오늘'은 전부 비어 있고, 첫 행동에 필요한 터미널이 리서치에만 있다." },
    { screen: "place", x: 50, y: 8, text: "관심종목이라는 별도 개체가 없다. 보는 것을 늘리는 행위 = pane을 놓는 행위. 명령 목록에 관심종목 명령을 더하지 않은 것이 이 결정의 증거다." },
    { screen: "place", x: 86, y: 32, text: "고정(핀)이 SWAP/PLACE를 가른다. 고정 안 된 pane은 다음 티커 명령이 내용을 바꾸고, 고정한 pane은 남는다 — 차트를 쌓지 않고 갈아끼우는 트레이딩 터미널의 습관." },
    { screen: "lists", x: 50, y: 6, text: "볼 종목의 출처 3종을 섞지 않고 pane의 출처로 나눈다. '관심종목이냐 전략 유니버스냐'가 앱의 규칙이 아니라 사용자의 배치 선택이 되는 지점." },
    { screen: "lists", x: 84, y: 70, text: "관찰 중 목록의 손익에도 참고용 라벨이 붙는다. 종료일 전 숫자는 어디에 나타나든 결과가 아니다." },
    { screen: "slots", x: 26, y: 78, text: "상폐 종목을 목록에서 지우지 않는다 — 상폐 포함이 데이터의 차별 요소다. 시세 자리를 비우고 '이력만'으로 표기해 백테스트용임을 말한다." },
    { screen: "slots", x: 82, y: 62, text: "배분 규칙이 '보이는 것이 쓴다' 하나여서 사용자가 슬롯을 관리할 일이 없다. 다만 잦은 구독 교체를 KIS가 견디는지는 구현에서 확인해야 한다." },
    { screen: "layout", x: 50, y: 40, text: "저장은 배치한 자리에서 일어난다(미결 3의 답 후보). 설정 화면은 이름 변경·삭제만 맡는다 — 만든 곳과 관리하는 곳을 나눈다." },
    { screen: "layout", x: 14, y: 20, text: "에이전트는 pane을 놓고 내용을 바꿀 수 있지만 배치를 흩뜨리거나 닫지 못한다. 사용자 배치의 소유권이 흔들리면 자기 화면이 아니게 된다." },
    { screen: "strat", x: 50, y: 8, text: "관찰이 도는 것이 아니라 물어볼 때 계산한다. 등록이 해시를 고정한 뒤의 데이터는 전부 out-of-sample이므로, 앱이 꺼져 있어도 시간이 증거를 만든다." },
    { screen: "strat", x: 30, y: 52, text: "수익률만 있으면 노이즈를 성적으로 읽는다. meanrev-krx의 -0.4%보다 '거래 3회'가 더 중요한 정보다 — 표본과 벤치마크가 정직성 규율의 일부인 이유." },
    { screen: "strat", x: 84, y: 78, text: "아무 때나 볼 수 있으면 좋아 보일 때 끊고 싶어진다. 사전등록이 고정하는 것은 해시와 종료일 둘이고, 종료일이 정지 시점을 묶는다. 중단 액션은 만들지 않는다." }
"""


def main():
    html = TEMPLATE.read_text(encoding="utf-8")
    html = html.replace("<title>주제 이름 — 플로우 목업</title>", f"<title>{TITLE}</title>")
    html = html.replace('<div class="deck-title">주제 이름', f'<div class="deck-title">{TITLE.split(" — ")[0]}')

    kit_block = ("/* ═══ [KIT] trdr ui-kit 인라인 (정본: artifacts/ui-kit/kit.css) ═══ */\n"
                 + KIT_CSS + MK_CSS + MK3_CSS + "\n/* ═══ /[KIT] ═══ */")
    html = re.sub(r"/\* ═══ \[KIT\].*?/\[KIT\] ═══ \*/", lambda m: kit_block, html, flags=re.S)

    screens = "\n".join([
        screen_empty(),
        arrow("티커 입력 — 005930"),
        screen_place(),
        arrow("목록 명령"),
        screen_lists(),
        arrow("종목이 41을 넘음"),
        screen_slots(),
        arrow("내 배치로 정리"),
        screen_layout(),
        arrow("전략 명령 — 오늘 프리셋"),
        screen_strat(),
    ])
    screens_block = "<!-- ═══ [SCREENS] ═══ -->\n" + screens + "\n<!-- ═══ /[SCREENS] ═══ -->"
    html = re.sub(r"<!-- ═══ \[SCREENS\].*?/\[SCREENS\] ═══ -->", lambda m: screens_block, html, flags=re.S)

    html = re.sub(r"var PRESET_NOTES = \[.*?\];",
                  lambda m: "var PRESET_NOTES = [" + PRESET_NOTES + "];", html, flags=re.S)
    html = html.replace('var MOCKUP_VERSION = "v1";', f'var MOCKUP_VERSION = "{VERSION}";')
    html = html.replace(
        "<li><b>v1</b> 최초 작성</li>",
        "<li><b>v3</b> 최초 작성 — '내 화면 만들기' 6프레임 (USER_JOURNEY P1 · SCREEN_ACTIONS 액션표). "
        "에이전트 프레임 2개는 w-b2v8x 뒤 v3.1로.</li>",
    )

    OUT.write_text(html, encoding="utf-8")
    print(f"wrote {OUT} ({len(html):,} bytes)")


if __name__ == "__main__":
    main()
