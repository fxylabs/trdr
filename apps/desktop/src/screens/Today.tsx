import { useNavigate } from "react-router";
import { Button, DataTable, EmptyState, EventList, Metric, Panel, StockCell } from "@trdr/ui";
import type { DataTableColumn, EventContent } from "@trdr/ui";

import type { AccountSummary, Holding, TodayEvent } from "../bindings";
import { SCREEN, TODAY_EVENT_KIND, words } from "../copy/ko";
import { clockTime, count, shares, signedPercent, signedWon, won } from "../copy/format";
import { todayRequest, useModel } from "../ipc/model";
import { paths } from "../shell/routes";
import { Screen } from "./Screen";
import { SectionNote, eventState, metricState, tableState, toneClass } from "./sections";

/**
 * What happened since the person last looked (`TodayModel/v1`).
 *
 * The recipe `contracts.v2.json` names for this view is TopBar, Metric,
 * DataTable with StockCell, EventList and a Button, and that is what is below.
 * Two of its rules are worth restating where they are obeyed: the account's
 * direction comes from `day_tone` rather than from the sign of `day_change`,
 * and a holdings section that is `stale` shows its rows with a note rather than
 * hiding them.
 */
export function Today()
{
    const navigate = useNavigate();
    const state = useModel(todayRequest());

    return (
        <Screen title={SCREEN.today.title} state={state}>
            {(model) => (
                <>
                    <AccountMetrics account={model.account} holdings={model.holdings.length} />

                    <SectionNote state={model.holdings_state} />
                    <Panel title={SCREEN.today.holdings} caption={SCREEN.today.holdingsCaption}>
                        <DataTable
                            label={SCREEN.today.holdings}
                            columns={HOLDING_COLUMNS}
                            rows={model.holdings}
                            rowId={(row) => row.symbol}
                            state={tableState(model.holdings_state)}
                            empty={
                                <EmptyState
                                    title={SCREEN.today.emptyHoldings}
                                    explanation={SCREEN.today.emptyHoldingsWhy}
                                />
                            }
                        />
                    </Panel>

                    <SectionNote state={model.events_state} />
                    <Panel title={SCREEN.today.events} caption={SCREEN.today.eventsCaption}>
                        <EventList
                            label={SCREEN.today.events}
                            items={model.events}
                            itemId={eventId}
                            event={eventContent}
                            state={eventState(model.events_state)}
                            empty={
                                <EmptyState
                                    title={SCREEN.today.emptyEvents}
                                    explanation={SCREEN.today.emptyEventsWhy}
                                />
                            }
                        />
                    </Panel>

                    <Button
                        variant="primary"
                        label={SCREEN.today.openLab}
                        onActivate={() => navigate(paths.lab)}
                    />
                </>
            )}
        </Screen>
    );
}

/** The four numbers the account is read through. */
function AccountMetrics(props: { readonly account: AccountSummary; readonly holdings: number })
{
    const { account, holdings } = props;
    const state = metricState(account.state);

    return (
        <div className="trdr-metric-grid">
            <Panel>
                <Metric label={SCREEN.today.totalValue} value={won(account.total_value)} state={state} />
            </Panel>
            <Panel>
                <Metric
                    label={SCREEN.today.dayChange}
                    value={signedWon(account.day_change)}
                    comparison={signedPercent(account.day_change_ratio)}
                    marketTone={account.day_tone}
                    state={state}
                />
            </Panel>
            <Panel>
                <Metric label={SCREEN.today.cash} value={won(account.cash)} state={state} />
            </Panel>
            <Panel>
                <Metric label={SCREEN.today.holdingCount} value={count(holdings)} state={state} />
            </Panel>
        </div>
    );
}

/** Gain or loss, with the direction the model reported and never a derived one. */
function Unrealized(props: { readonly holding: Holding })
{
    const { holding } = props;
    const tone = toneClass(holding.tone);

    return (
        <>
            <span className={tone}>{signedWon(holding.unrealized)}</span>
            <span className="trdr-data-sub">{signedPercent(holding.unrealized_ratio)}</span>
        </>
    );
}

/** The identity column first, as `DataTable`'s rule requires. */
const HOLDING_COLUMNS: readonly DataTableColumn<Holding>[] = [
    {
        id: "symbol",
        header: SCREEN.today.columnSymbol,
        width: "40%",
        cell: (row) => (
            <StockCell
                symbol={row.symbol.slice(0, 3)}
                name={row.name}
                summary={SCREEN.today.holdingSummary(
                    row.symbol,
                    shares(row.quantity),
                    won(row.average_price)
                )}
            />
        )
    },
    {
        id: "marketValue",
        header: SCREEN.today.columnMarketValue,
        cell: (row) => won(row.market_value)
    },
    {
        id: "unrealized",
        header: SCREEN.today.columnUnrealized,
        cell: (row) => <Unrealized holding={row} />
    },
    {
        id: "lastPrice",
        header: SCREEN.today.columnLastPrice,
        cell: (row) => won(row.last_price)
    }
];

/** Events carry no id of their own, so one is made from what identifies them. */
function eventId(event: TodayEvent): string
{
    return `${event.kind}:${event.at}:${event.title}`;
}

/** One event as the three parts `EventList` draws. The summary is a code. */
function eventContent(event: TodayEvent): EventContent
{
    return {
        icon: TODAY_EVENT_KIND[event.kind],
        title: event.title,
        summary: `${clockTime(event.at)} · ${words(event.summary)}`
    };
}
