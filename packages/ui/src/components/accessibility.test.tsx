/**
 * The accessibility line of each contract, one assertion at a time.
 *
 * These are not extras. `aria-current="page"` is what draws the lime marker in
 * the sidebar, native table semantics are what make a column header a column
 * header, and `role="alert"` is the difference between an error the user sees
 * and one they hit again. Each test below names the contract line it holds.
 */

import { cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, expect, test } from "vitest";

import {
    AppShell,
    BrokerConnection,
    Button,
    DataTable,
    DayProgress,
    EmptyState,
    EventList,
    Field,
    InlineFeedback,
    NavigationItem,
    Panel,
    Sidebar,
    SignalLog,
    StockCell,
    TopBar
} from "../index";
import { HOLDINGS, HOLDING_COLUMNS, SIGNALS, eventContent, EVENTS, signalContent } from "../testing/catalogue";

afterEach(cleanup);

/** AppShell: one application landmark; sidebar and terminal named separately. */
test("the shell names its three regions and has one main", () =>
{
    render(
        <AppShell
            activeArea="today"
            sidebar={<Sidebar items={[{ id: "today", label: "Today", current: true }]} label="구역" />}
            agentRail={<div data-testid="rail" />}
        >
            <p>work</p>
        </AppShell>
    );

    expect(screen.getAllByRole("main")).toHaveLength(1);
    expect(screen.getByRole("navigation", { name: "구역" })).toBeDefined();
    expect(screen.getByRole("region", { name: "Agent terminal" })).toBeDefined();
});

/** AppShell: the rail is a slot, so navigating the centre cannot remount it. */
test("changing the centre view leaves the rail's node alone", () =>
{
    const rail = <div data-testid="rail" />;
    const { rerender } = render(
        <AppShell activeArea="today" sidebar={<Sidebar items={[]} />} agentRail={rail}>
            <p>today</p>
        </AppShell>
    );

    const before = screen.getByTestId("rail");

    rerender(
        <AppShell activeArea="lab" sidebar={<Sidebar items={[]} />} agentRail={rail}>
            <p>lab</p>
        </AppShell>
    );

    expect(screen.getByTestId("rail")).toBe(before);
    expect(screen.getByText("lab")).toBeDefined();
});

/** NavigationItem: link or button semantics, aria-current on the current one. */
test("the current section is the one marked current, and only it", () =>
{
    render(
        <Sidebar
            items={[
                { id: "today", label: "Today", href: "#/", current: true },
                { id: "lab", label: "Lab", href: "#/lab" }
            ]}
        />
    );

    expect(screen.getByRole("link", { name: "Today" }).getAttribute("aria-current")).toBe("page");
    expect(screen.getByRole("link", { name: "Lab" }).getAttribute("aria-current")).toBeNull();
});

test("a navigation item with no destination is still a control", () =>
{
    render(<NavigationItem label="Lab" onActivate={() => undefined} />);

    expect(screen.getByRole("button", { name: "Lab" })).toBeDefined();
});

/** BrokerConnection: the state is in the text, not in the colour alone. */
test("the connection says what it is in words", () =>
{
    render(
        <BrokerConnection broker="KIS" state="stale" stateLabel="지연됨" authority="읽기 전용" lastSync="14:31 동기화" />
    );

    expect(screen.getByText(/지연됨/)).toBeDefined();
    expect(screen.getByText(/읽기 전용/)).toBeDefined();
});

/** TopBar: the title is the view's h1; the back control names its destination. */
test("the view has one h1 and a back control that says where it goes", () =>
{
    render(<TopBar title="전략 상세" back={{ label: "전략 목록", onActivate: () => undefined }} />);

    expect(screen.getByRole("heading", { level: 1, name: "전략 상세" })).toBeDefined();
    expect(screen.getByRole("button", { name: /전략 목록/ })).toBeDefined();
});

/** Panel: a named region is a section with its heading attached to it. */
test("a titled panel is a region a reader can jump to", () =>
{
    render(
        <Panel title="실행 규칙" caption="strategy.yaml">
            <p>rules</p>
        </Panel>
    );

    expect(screen.getByRole("region", { name: "실행 규칙" })).toBeDefined();
});

test("an untitled panel claims no landmark it cannot name", () =>
{
    const { container } = render(
        <Panel>
            <p>plain</p>
        </Panel>
    );

    expect(container.querySelector("section")).toBeNull();
    expect(container.querySelector(".trdr-panel-header")).toBeNull();
});

/** Button: native button, programmatic disabled. */
test("a disabled button is disabled rather than dimmed", () =>
{
    render(<Button label="비활성" disabled />);

    const button = screen.getByRole("button", { name: "비활성" });

    expect(button.tagName).toBe("BUTTON");
    expect((button as HTMLButtonElement).disabled).toBe(true);
});

/** Field: explicit label, aria-invalid, described error, no placeholder-only label. */
test("a field is labelled, and says so when it is invalid", () =>
{
    render(<Field label="관측 일수" value="0" hint="1일 이상" error="1일 이상이어야 합니다." />);

    const input = screen.getByLabelText("관측 일수");

    expect(input.getAttribute("aria-invalid")).toBe("true");
    expect(input.getAttribute("placeholder")).toBeNull();

    const described = (input.getAttribute("aria-describedby") ?? "").split(" ");
    const text = described.map((id) => document.getElementById(id)?.textContent);

    expect(text).toContain("1일 이상이어야 합니다.");
});

/** DataTable: native table semantics, headers declare sort, numerals are tabular. */
test("the table is a table, its headers are headers, and its numbers are numbers", () =>
{
    render(
        <DataTable
            label="보유 종목"
            columns={HOLDING_COLUMNS}
            rows={HOLDINGS}
            rowId={(row) => row.id}
            sort={{ columnId: "profit", direction: "descending" }}
            onSort={() => undefined}
        />
    );

    const table = screen.getByRole("table", { name: "보유 종목" });

    expect(within(table).getAllByRole("columnheader")).toHaveLength(4);
    expect(within(table).getAllByRole("row")).toHaveLength(4);
    expect(within(table).getByRole("columnheader", { name: "총손익" }).getAttribute("aria-sort")).toBe("descending");
    expect(within(table).getByRole("columnheader", { name: "평가금액" }).getAttribute("aria-sort")).toBe("none");
    expect(within(table).getByRole("columnheader", { name: "종목" }).getAttribute("aria-sort")).toBeNull();
});

/** DataTable: a sortable header is reachable by keyboard, not only by pointer. */
test("sorting is a control", async () =>
{
    const sorted: string[] = [];

    render(
        <DataTable
            label="보유 종목"
            columns={HOLDING_COLUMNS}
            rows={HOLDINGS}
            rowId={(row) => row.id}
            onSort={(column) => sorted.push(column)}
        />
    );

    screen.getByRole("button", { name: "총손익" }).click();

    expect(sorted).toEqual(["profit"]);
});

/** StockCell: the symbol tile is decorative when the name is beside it. */
test("the symbol tile is not read out twice", () =>
{
    const { container } = render(<StockCell symbol="삼성" name="삼성전자" summary="40주" />);

    expect(container.querySelector(".trdr-symbol")?.getAttribute("aria-hidden")).toBe("true");
    expect(screen.getByText("삼성전자")).toBeDefined();
});

/** EventList: list semantics, and the event's type in text. */
test("the event list is a list and each event says what kind it is", () =>
{
    render(<EventList label="오늘의 이벤트" items={EVENTS} itemId={(item) => item.id} event={eventContent} />);

    const list = screen.getByRole("list", { name: "오늘의 이벤트" });

    expect(within(list).getAllByRole("listitem")).toHaveLength(3);
    expect(within(list).getByText("공시")).toBeDefined();
});

/** SignalLog: chronological list semantics with a machine-readable timestamp. */
test("every signal carries a datetime and the rule it matched", () =>
{
    const { container } = render(
        <SignalLog label="신호 기록" signals={SIGNALS} signalId={(record) => record.id} signal={signalContent} />
    );

    expect(within(screen.getByRole("list", { name: "신호 기록" })).getAllByRole("listitem")).toHaveLength(2);
    expect(container.querySelector("time")?.getAttribute("datetime")).toBe("2026-08-11T14:31:00+09:00");
    expect(screen.getByText(/MA20 < MA60/)).toBeDefined();
});

/** DayProgress: elapsed and total are exposed as values, not only as segments. */
test("the day bar reports its own numbers", () =>
{
    render(<DayProgress elapsed={8} total={20} today={8} label="20일 중 8일 관측" />);

    const bar = screen.getByRole("progressbar", { name: "20일 중 8일 관측" });

    expect(bar.getAttribute("aria-valuenow")).toBe("8");
    expect(bar.getAttribute("aria-valuemax")).toBe("20");
});

/** InlineFeedback: errors alert, everything else is polite. */
test("an error announces itself and a success does not interrupt", () =>
{
    const { rerender } = render(<InlineFeedback tone="error" message="연결 실패" />);

    expect(screen.getByRole("alert").textContent).toContain("연결 실패");

    rerender(<InlineFeedback tone="success" message="동기화 완료" />);

    expect(screen.getByRole("status").textContent).toContain("동기화 완료");
});

/** EmptyState: the action stays keyboard reachable. */
test("an empty state's next step is a real control", () =>
{
    render(
        <EmptyState title="아직 없습니다" explanation="실험실에서 시작합니다." action={<Button label="실험실 열기" />} />
    );

    expect(screen.getByRole("button", { name: "실험실 열기" })).toBeDefined();
});
