/**
 * The `rules` line of each contract — the constraints, not the styling.
 *
 * A rule here is a claim the product makes: that colour on a number means the
 * market moved, that a frozen strategy cannot be edited, that a paper return is
 * not the headline of a validation that is eight days into twenty. They are the
 * things most likely to be undone by a well-meaning change, so they are asserted
 * rather than written down in a comment.
 */

import { cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, expect, test } from "vitest";

import { CoverageCard, DataTable, DayProgress, Metric, RuleGrid, StockCell, StrategyCard, Tag } from "../index";
import { CATALOGUE, HOLDINGS, HOLDING_COLUMNS, RULES, STRATEGY } from "../testing/catalogue";
import { CONTRACTS } from "../testing/contractSource";

afterEach(cleanup);

/**
 * Panel: a grouping surface, not a default wrapper.
 *
 * The failure this rules out is the one that arrives component by component —
 * each one wrapping itself in a panel "so it looks finished", until every screen
 * is a grid of boxes and the border has stopped meaning anything. Only the two
 * roles whose contract class *is* `trdr-panel` may render one.
 */
test("no component wraps itself in a panel", () =>
{
    const allowed = Object.entries(CONTRACTS.components)
        .filter(([, contract]) => contract.class === "trdr-panel")
        .map(([name]) => name);

    expect(allowed.sort()).toEqual(["CoverageCard", "Panel"]);

    for (const role of CATALOGUE.filter((one) => !allowed.includes(one.contract)))
    {
        const { container } = render(role.cases[0]!.element);

        expect([role.contract, container.querySelectorAll(".trdr-panel").length]).toEqual([role.contract, 0]);
        cleanup();
    }
});

/** Metric: market colours apply only to values. */
test("a market tone lands on the number and on nothing else", () =>
{
    const { container } = render(
        <Metric label="검증 수익률" value="+8.4%" comparison="학습 +15.7%" marketTone="up" />
    );

    expect(container.querySelector("strong")?.className).toContain("trdr-market-up");
    expect(container.querySelector("label")?.className ?? "").not.toContain("trdr-market");
    expect(container.querySelector("small")?.className ?? "").not.toContain("trdr-market");
});

/** Metric: tabular numerals, in every state that has a number in it. */
test("a metric's value carries tabular figures", () =>
{
    const { container } = render(<Metric label="승률" value="54.2%" />);

    expect(container.querySelector("strong")?.className).toContain("trdr-num");
});

/** DataTable: the first column is the identity column. */
test("the identity column holds a name and the rest hold numbers", () =>
{
    const { container } = render(
        <DataTable label="보유 종목" columns={HOLDING_COLUMNS} rows={HOLDINGS} rowId={(row) => row.id} />
    );

    const cells = [...(container.querySelector("tbody tr")?.querySelectorAll("td") ?? [])];

    expect(cells).toHaveLength(4);
    expect(cells[0]!.className).not.toContain("trdr-num");
    expect(cells[0]!.querySelector(".trdr-stock-cell")).not.toBeNull();

    for (const cell of cells.slice(1))
    {
        expect(cell.className).toContain("trdr-num");
    }
});

/** DataTable: a column can opt out, so a text column is not forced into figures. */
test("a column that is not a number can say so", () =>
{
    const { container } = render(
        <DataTable
            label="실행 기록"
            columns={[
                { id: "run", header: "실행", cell: () => "run-01" },
                { id: "note", header: "메모", numeric: false, cell: () => "재현 확인" }
            ]}
            rows={[{ id: "run-01" }]}
            rowId={(row) => row.id}
        />
    );

    const cells = [...(container.querySelector("tbody tr")?.querySelectorAll("td") ?? [])];

    expect(cells[1]!.className).not.toContain("trdr-num");
});

/** RuleGrid: frozen rules cannot show an edit affordance. */
test("a frozen rule grid offers no way to edit it", () =>
{
    const { container, rerender } = render(
        <RuleGrid {...RULES} state="draft" onEdit={() => undefined} editLabel="수정" />
    );

    expect(container.querySelectorAll("button")).toHaveLength(4);

    rerender(<RuleGrid {...RULES} state="frozen" onEdit={() => undefined} editLabel="수정" />);

    expect(container.querySelectorAll("button")).toHaveLength(0);
});

/** CoverageCard: the progress track is supplementary, not the only value. */
test("coverage is a number first and a bar second", () =>
{
    render(
        <CoverageCard
            source="KIS 일봉"
            days={1258}
            covered={1190}
            total={1258}
            summary="1,190 / 1,258일 · 94.6% · 68일 결측"
            state="incomplete"
        />
    );

    expect(screen.getByText("1190 / 1258")).toBeDefined();
    expect(screen.getByText(/94\.6%/)).toBeDefined();
    expect(screen.getByRole("progressbar").getAttribute("aria-valuetext")).toContain("94.6%");
});

/**
 * StrategyCard: performance never outranks observation progress and deviations.
 *
 * Order is the assertion. Observation comes first, the return sits between its
 * two neighbours at the same size, and it is never given a market colour — a
 * green +2.1% on day eight of twenty is a claim the data has not earned.
 */
test("the paper return does not lead the card", () =>
{
    const { container } = render(<StrategyCard strategy={STRATEGY} href="#/s" />);

    const stats = [...container.querySelectorAll(".trdr-strategy-stat")];
    const labels = stats.map((stat) => stat.querySelector("label")?.textContent);

    expect(labels).toEqual(["관측", "모의 수익률", "상태"]);
    expect(stats[1]!.querySelector("strong")?.className ?? "").not.toContain("trdr-market");
    expect(container.querySelector(".trdr-strategy-name h3")?.textContent).toBe(STRATEGY.name);
});

/** StrategyCard: the whole card is one link or button. */
test("the card is a single control, whichever kind it is", () =>
{
    const { container, rerender } = render(<StrategyCard strategy={STRATEGY} href="#/s" />);

    expect(container.querySelectorAll("a")).toHaveLength(1);
    expect(container.querySelectorAll("button")).toHaveLength(0);

    rerender(<StrategyCard strategy={STRATEGY} onOpen={() => undefined} />);

    expect(container.querySelectorAll("button")).toHaveLength(1);
    expect(within(screen.getByRole("button")).getByRole("heading", { level: 3 })).toBeDefined();
});

/** DayProgress: today is one segment, the days before it are done, the rest are not. */
test("the day bar marks today once", () =>
{
    const { container } = render(<DayProgress elapsed={8} total={20} today={8} label="20일 중 8일" />);

    expect(container.querySelectorAll("i")).toHaveLength(20);
    expect(container.querySelectorAll('i[data-state="done"]')).toHaveLength(7);
    expect(container.querySelectorAll('i[data-state="today"]')).toHaveLength(1);
});

test("a completed window has no today and no gaps", () =>
{
    const { container } = render(<DayProgress elapsed={20} total={20} state="completed" label="20일 중 20일" />);

    expect(container.querySelectorAll('i[data-state="done"]')).toHaveLength(20);
    expect(container.querySelectorAll('i[data-state="today"]')).toHaveLength(0);
});

/** StockCell: no external logo dependency. */
test("an instrument is drawn with characters, not a fetched image", () =>
{
    const { container } = render(<StockCell symbol="삼성" name="삼성전자" />);

    expect(container.querySelectorAll("img")).toHaveLength(0);
    expect(container.querySelector(".trdr-symbol")?.textContent).toBe("삼성");
});

/** Tag: tags report state; they are not buttons. */
test("a tag cannot be pressed", () =>
{
    const { container } = render(<Tag tone="warning" label="조건 이탈" />);

    expect(container.firstElementChild?.tagName).toBe("SPAN");
    expect(container.querySelectorAll("button")).toHaveLength(0);
});
