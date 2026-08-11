export type StockCellProps =
{
    /** Two or three characters for the tile. `삼성`, `NAV`. */
    symbol: string;
    name: string;
    /** The position in one line. `40주 · 평균 71,250원`. */
    summary?: string;
};

/**
 * The identity of a row: which instrument this line is about.
 *
 * The tile is letters on a neutral square and never a logo — Phase 1/2 fetches
 * nothing from a third party to draw a table — and with the name right beside it
 * the tile is decoration, so it is hidden from the reading order rather than
 * read out twice.
 */
export function StockCell(props: StockCellProps)
{
    const { symbol, name, summary } = props;

    return (
        <div className="trdr-stock-cell" data-state="ready">
            <span className="trdr-symbol" aria-hidden="true">
                {symbol}
            </span>

            <span>
                <b>{name}</b>
                {summary === undefined ? null : <small>{summary}</small>}
            </span>
        </div>
    );
}
