import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { RouterProvider, createMemoryRouter } from "react-router";
import { afterEach, beforeEach, expect, test, vi } from "vitest";

import { commands } from "../bindings";
import { ERROR, ORIGIN, SCREEN, SUPPORT_HEADLINE, VERDICT_HEADLINE } from "../copy/ko";
import {
    fitAddonModule,
    tauriCoreModule,
    xtermModule,
    xtermStylesModule
} from "../terminal/doubles";
import { paths, routes, strategyPath } from "../shell/routes";
import {
    LAB_DRAFT,
    LAB_RESULT,
    STRATEGIES,
    STRATEGY_DETAIL,
    TODAY,
    answers,
    refuses
} from "./fixtures";

/**
 * The host, replaced.
 *
 * Everything below `commands` is Tauri's `invoke`, which needs a WebView with a
 * host behind it and has none here. What is being tested is what a screen does
 * with an answer, so the answer is supplied.
 */
vi.mock("../bindings", () => ({
    commands: {
        ping: vi.fn(),
        bootstrapGet: vi.fn(),
        todayGet: vi.fn(),
        labDraftGet: vi.fn(),
        backtestGet: vi.fn(),
        strategiesList: vi.fn(),
        strategyGet: vi.fn(),
        terminalStart: vi.fn(),
        terminalInput: vi.fn(),
        terminalResize: vi.fn(),
        terminalRestart: vi.fn()
    }
}));

/**
 * The terminal, without a terminal. See `terminal/doubles.ts` for why the rail
 * is replaced in a test that renders the route table, and where its own
 * behaviour is held instead.
 */
vi.mock("@xterm/xterm", () => xtermModule());
vi.mock("@xterm/addon-fit", () => fitAddonModule());
vi.mock("@xterm/xterm/css/xterm.css", () => xtermStylesModule());
vi.mock("@tauri-apps/api/core", () => tauriCoreModule());

const WORKSPACE = "01KZNP0GQ3X8ARK9DQ489Z7WJ8";

beforeEach(() =>
{
    // The rail is scenery here, not the subject — see `terminal/doubles.ts`. Its
    // commands still have to answer, because the session calls `.catch` on what
    // they return and a bare `vi.fn()` returns nothing.
    vi.mocked(commands.terminalStart).mockImplementation((id) =>
        Promise.resolve({
            v: 1,
            id,
            outcome: {
                status: "ok",
                value: { agent: "claude", pid: 4242, process: "ready", cols: 80, rows: 24 }
            }
        })
    );

    for (const command of [
        commands.terminalInput,
        commands.terminalResize,
        commands.terminalRestart
    ])
    {
        vi.mocked(command).mockImplementation((id: string) =>
            Promise.resolve({ v: 1, id, outcome: { status: "ok", value: { process: "ready" } } })
        );
    }

    vi.mocked(commands.ping).mockImplementation((id) =>
        Promise.resolve({ v: 1, id, outcome: { status: "ok", value: { protocol_version: 1 } } })
    );
    vi.mocked(commands.bootstrapGet).mockImplementation((id) =>
        Promise.resolve({
            v: 1,
            id,
            outcome: {
                status: "ok",
                value: {
                    protocol_version: 1,
                    app_version: "9.9.9",
                    workspace_id: WORKSPACE,
                    workspace_path: "/private/tmp/trdr-t/x/workspaces/default",
                    schema_version: 1
                }
            }
        })
    );
    vi.mocked(commands.todayGet).mockImplementation(answers(TODAY));
    vi.mocked(commands.labDraftGet).mockImplementation(answers(LAB_DRAFT));
    vi.mocked(commands.backtestGet).mockImplementation(answers(LAB_RESULT));
    vi.mocked(commands.strategiesList).mockImplementation(answers(STRATEGIES));
    vi.mocked(commands.strategyGet).mockImplementation((id) => answers(STRATEGY_DETAIL)(id));
});

afterEach(() =>
{
    document.body.innerHTML = "";
    vi.resetAllMocks();
});

/** The whole application at one path, through the route table the app uses. */
function open(path: string)
{
    return render(<RouterProvider router={createMemoryRouter(routes, { initialEntries: [path] })} />);
}

/** Every screen begins by asking, so every test begins by waiting for an answer. */
async function openAndSettle(path: string)
{
    const view = open(path);

    await screen.findByText(ORIGIN.synthetic.statement);

    return view;
}

test("Today renders the account, its holdings and its events", async () =>
{
    await openAndSettle(paths.today);

    expect(screen.getByRole("heading", { level: 1, name: SCREEN.today.title })).toBeDefined();
    expect(screen.getByText("12,480,000원")).toBeDefined();
    expect(screen.getByText("+142,000원")).toBeDefined();
    expect(screen.getByText("+1.15%")).toBeDefined();

    const holdings = screen.getByRole("table", { name: SCREEN.today.holdings });

    expect(within(holdings).getByText("합성전자")).toBeDefined();
    expect(within(holdings).getByText("3,248,000원")).toBeDefined();

    const events = screen.getByRole("list", { name: SCREEN.today.events });

    expect(within(events).getByText("주요사항보고서 (합성 공시)")).toBeDefined();
    expect(within(events).getByText(/보유 종목에 관한 공시가 도착했습니다\./)).toBeDefined();
});

/**
 * The market convention, and the one thing a screen must never derive.
 *
 * The second holding gained money and the model calls it `down`. A screen that
 * read the direction off the sign would paint it red — which in this market is
 * the colour for a gain, so the mistake would look right in exactly the case
 * where it is wrong.
 */
test("a value's colour comes from the model's tone and never from its sign", async () =>
{
    await openAndSettle(paths.today);

    const gained = screen.getByText("+112,000원");
    const alsoGainedButMarkedDown = screen.getByText("+4,000원");

    expect(gained.className).toBe("trdr-market-up");
    expect(alsoGainedButMarkedDown.className).toBe("trdr-market-down");
});

/** The exact contract spellings, one of which is kebab where the rest are not. */
test("the broker connection carries the contract's own spelling", async () =>
{
    await openAndSettle(paths.today);

    const connection = document.querySelector(".trdr-connection");

    expect(connection?.getAttribute("data-state")).toBe("connected-read-only");
    expect(screen.getByText(/연결됨/)).toBeDefined();
});

test("the Lab draft states its support, its rules and which days are missing", async () =>
{
    await openAndSettle(paths.lab);

    expect(screen.getByRole("heading", { level: 2, name: SUPPORT_HEADLINE["missing-data"] })).toBeDefined();
    expect(screen.getByText("시장 · SYN")).toBeDefined();
    expect(screen.getByText(/2026-06-15 ~ 2026-06-17/)).toBeDefined();
    expect(screen.getByText(/2026-07-28 ~ 2026-07-29/)).toBeDefined();
});

/**
 * A stale section shows its numbers.
 *
 * The draft's coverage comes back `stale` because the period has gaps. That
 * means the coverage is incomplete, not absent — the count has to be on screen
 * beside the admission that it is short.
 */
test("a stale section renders its data and says that it is incomplete", async () =>
{
    await openAndSettle(paths.lab);

    expect(screen.getByText("63 / 68")).toBeDefined();
    expect(screen.getByText(/93%/)).toBeDefined();
    expect(screen.getAllByRole("status").some((node) => node.textContent === "지연된 값")).toBe(true);
});

test("the Lab result states its verdict, its metrics and its warnings", async () =>
{
    await openAndSettle(paths.labResult);

    expect(screen.getByRole("heading", { level: 2, name: VERDICT_HEADLINE.qualified })).toBeDefined();
    expect(screen.getByText("+4.12%")).toBeDefined();
    expect(screen.getByText("8.70%")).toBeDefined();
    expect(screen.getByText("0.62")).toBeDefined();
    expect(screen.getByText(/2026-06-15 ~ 2026-06-17/)).toBeDefined();
});

test("the strategies list ranks observation before the return the card carries", async () =>
{
    await openAndSettle(paths.strategies);

    const [card] = screen.getAllByRole("link", { name: /합성 평균회귀/ });
    const statistics = card?.querySelectorAll(".trdr-strategy-stat label") ?? [];

    expect([...statistics].map((node) => node.textContent)).toEqual([
        SCREEN.strategies.observation,
        SCREEN.strategies.paperReturn,
        SCREEN.strategies.currentState
    ]);
    expect(within(card as HTMLElement).getByText("9 / 60일")).toBeDefined();
    expect(within(card as HTMLElement).getByText("+1.80%")).toBeDefined();
});

test("opening a strategy navigates to its own address and back again", async () =>
{
    const user = userEvent.setup();
    await openAndSettle(paths.strategies);

    await user.click(screen.getAllByRole("link", { name: /합성 평균회귀/ })[0] as HTMLElement);
    await screen.findByRole("heading", { level: 1, name: STRATEGY_DETAIL.name });

    expect(vi.mocked(commands.strategyGet).mock.calls[0]?.[1]).toBe("syn-meanrev");

    await user.click(screen.getByRole("button", { name: `← ${SCREEN.strategyDetail.back}` }));

    expect(await screen.findByRole("heading", { level: 1, name: SCREEN.strategies.title })).toBeDefined();
});

test("the strategy detail shows frozen rules with no way to edit them", async () =>
{
    await openAndSettle(strategyPath("syn-meanrev"));

    const grid = document.querySelector(".trdr-rule-grid");

    expect(grid?.getAttribute("data-state")).toBe("frozen");
    expect(grid?.querySelector("button")).toBeNull();
});

test("the strategy detail logs every signal with the rule behind it", async () =>
{
    await openAndSettle(strategyPath("syn-meanrev"));

    const log = screen.getByRole("list", { name: SCREEN.strategyDetail.signalLog });

    expect(within(log).getByText("진입 · SYN0002")).toBeDefined();
    expect(within(log).getByText(/규칙: 5일 연속 하락/)).toBeDefined();
});

/**
 * Nothing to show is not the same fact as something went wrong.
 *
 * The fixture has data in every array, so the empty case only exists if a test
 * writes one. A person with no registered strategies is told what to do next; a
 * failed request gets an alert with no invitation attached.
 */
test("an empty list explains the next step rather than reporting a failure", async () =>
{
    vi.mocked(commands.strategiesList).mockImplementation(
        answers({ ...STRATEGIES, strategies: [], state: "empty" })
    );

    await openAndSettle(paths.strategies);

    expect(screen.getByText(SCREEN.strategies.empty)).toBeDefined();
    expect(screen.getByRole("button", { name: SCREEN.today.openLab })).toBeDefined();
    expect(screen.queryAllByRole("alert")).toHaveLength(0);
});

test("every screen survives a model whose every array is empty", async () =>
{
    vi.mocked(commands.todayGet).mockImplementation(
        answers({
            ...TODAY,
            holdings: [],
            holdings_state: "empty",
            events: [],
            events_state: "empty"
        })
    );
    vi.mocked(commands.labDraftGet).mockImplementation(
        answers({ ...LAB_DRAFT, coverage: [], coverage_state: "empty" })
    );
    vi.mocked(commands.backtestGet).mockImplementation(
        answers({ ...LAB_RESULT, curve: [], warnings: [] })
    );
    vi.mocked(commands.strategyGet).mockImplementation((id) =>
        answers<typeof STRATEGY_DETAIL>({
            ...STRATEGY_DETAIL,
            positions: [],
            positions_state: "empty",
            signals: [],
            signals_state: "empty",
            deviations: []
        })(id)
    );

    await openAndSettle(paths.today);
    expect(screen.getByText(SCREEN.today.emptyHoldings)).toBeDefined();
    expect(screen.getByText(SCREEN.today.emptyEvents)).toBeDefined();

    document.body.innerHTML = "";
    await openAndSettle(paths.lab);
    expect(screen.getByText(SCREEN.labDraft.emptyCoverage)).toBeDefined();

    document.body.innerHTML = "";
    await openAndSettle(paths.labResult);
    expect(screen.getByText(SCREEN.labResult.curveEmpty)).toBeDefined();

    document.body.innerHTML = "";
    await openAndSettle(strategyPath("syn-meanrev"));
    expect(screen.getByText(SCREEN.strategyDetail.emptyPositions)).toBeDefined();
    expect(screen.getByText(SCREEN.strategyDetail.emptySignals)).toBeDefined();
    expect(screen.getByText(SCREEN.strategyDetail.emptyDeviations)).toBeDefined();
});

/** A refusal is an envelope and reaches the person as a sentence, not a code. */
test("a refused command is read out of its envelope and said in Korean", async () =>
{
    vi.mocked(commands.strategyGet).mockImplementation((id) =>
        refuses<typeof STRATEGY_DETAIL>("DATA_INCOMPLETE")(id)
    );

    open(strategyPath("not-a-strategy"));

    expect(await screen.findByText(new RegExp(ERROR.DATA_INCOMPLETE))).toBeDefined();
});

/**
 * A model can arrive with the wrong contract.
 *
 * A packaged app can meet a newer host. Rendering whichever fields happened to
 * line up would be worse than saying nothing, so the screen refuses.
 */
test("a model naming another contract is refused rather than rendered", async () =>
{
    vi.mocked(commands.todayGet).mockImplementation(
        answers({ ...TODAY, model: "TodayModel/v2" })
    );

    open(paths.today);

    expect(await screen.findByText(new RegExp(ERROR.DATA_UNSUPPORTED))).toBeDefined();
    expect(screen.queryByText("12,480,000원")).toBeNull();
});

/** An answer to another request is a correlation bug, and is shown as one. */
test("an envelope naming a different request is refused", async () =>
{
    vi.mocked(commands.todayGet).mockImplementation(() =>
        Promise.resolve({
            v: 1,
            id: "01KZNNR5X818P3J6ENYKSADP8W",
            outcome: { status: "ok" as const, value: TODAY }
        })
    );

    open(paths.today);

    expect(await screen.findByText(new RegExp(ERROR.APP_PROTOCOL_VERSION))).toBeDefined();
});

/** No host at all is the one case that is genuinely a rejected promise. */
test("a command with no host behind it is named rather than left loading", async () =>
{
    vi.mocked(commands.labDraftGet).mockRejectedValue(new Error("there is no host here"));

    open(paths.lab);

    expect(await screen.findByText(new RegExp(ERROR.APP_NOT_RUNNING))).toBeDefined();
});
