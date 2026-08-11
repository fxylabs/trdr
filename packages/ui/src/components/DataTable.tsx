import type { ReactNode } from "react";

export type DataTableState = "loading" | "ready" | "empty" | "stale" | "error";

export type DataTableColumn<Row> =
{
    id: string;
    header: string;
    /** `table-layout: fixed` needs a width on the identity column at least. */
    width?: string;
    /**
     * Tabular figures. Defaults to true everywhere except the first column,
     * which is the identity column and holds a name rather than a number.
     */
    numeric?: boolean;
    sortable?: boolean;
    cell: (row: Row) => ReactNode;
};

export type DataTableSort =
{
    columnId: string;
    direction: "ascending" | "descending";
};

export type DataTableProps<Row> =
{
    /** The table's accessible name. Holdings, signals, runs — say which. */
    label: string;
    /** The first one is the identity column: what each row is. */
    columns: readonly DataTableColumn<Row>[];
    rows: readonly Row[];
    rowId: (row: Row) => string;
    sort?: DataTableSort;
    onSort?: (columnId: string) => void;
    state?: DataTableState;
    /** An `<EmptyState/>`. Shown across the full width when there is nothing. */
    empty?: ReactNode;
};

/** Whether a column's cells carry tabular figures. See `numeric`. */
function isNumeric<Row>(column: DataTableColumn<Row>, index: number): boolean
{
    return column.numeric ?? index > 0;
}

function HeaderCell<Row>(props: {
    column: DataTableColumn<Row>;
    sort: DataTableSort | undefined;
    onSort: ((columnId: string) => void) | undefined;
})
{
    const { column, sort, onSort } = props;
    const sortable = column.sortable === true && onSort !== undefined;
    const direction = sort?.columnId === column.id ? sort.direction : "none";

    return (
        <th scope="col" aria-sort={column.sortable === true ? direction : undefined}>
            {sortable ? (
                <button className="trdr-inline-action" type="button" onClick={() => onSort(column.id)}>
                    {column.header}
                </button>
            ) : (
                column.header
            )}
        </th>
    );
}

/** Three rows of nothing, so the table keeps its height while it waits. */
function LoadingRows(props: { columns: number })
{
    return (
        <>
            {[0, 1, 2].map((row) => (
                <tr key={row}>
                    {Array.from({ length: props.columns }, (_unused, cell) => (
                        <td key={cell}>
                            <i className="trdr-skeleton trdr-skeleton-line" />
                        </td>
                    ))}
                </tr>
            ))}
        </>
    );
}

function DataRows<Row>(props: Pick<DataTableProps<Row>, "columns" | "rows" | "rowId">)
{
    const { columns, rows, rowId } = props;

    return (
        <>
            {rows.map((row) => (
                <tr key={rowId(row)}>
                    {columns.map((column, index) => (
                        <td key={column.id} className={isNumeric(column, index) ? "trdr-num" : undefined}>
                            {column.cell(row)}
                        </td>
                    ))}
                </tr>
            ))}
        </>
    );
}

/**
 * Rows of numbers about things.
 *
 * Generic over the row, deliberately: Today lists holdings, the strategy detail
 * lists signals, and the Lab lists runs. They share this component because a
 * column knows how to render one row and the table knows nothing else about it.
 * A table that had learned what a holding is would be forked by the second
 * screen that needed it.
 *
 * Two of the contract's rules are structural rather than cosmetic. Rows are
 * 52px, from `--trdr-size-row-data`, so a dense table stays scannable at 1440.
 * And the first column is the identity column — what the row is about — which is
 * why it is the one column the stylesheet aligns left and the one this component
 * does not put tabular figures on by default.
 */
export function DataTable<Row>(props: DataTableProps<Row>)
{
    const { label, columns, rows, rowId, sort, onSort, state = "ready", empty } = props;

    return (
        <table className="trdr-data-table" data-state={state} aria-label={label}>
            <colgroup>
                {columns.map((column) => (
                    <col key={column.id} {...(column.width === undefined ? {} : { style: { width: column.width } })} />
                ))}
            </colgroup>

            <thead>
                <tr>
                    {columns.map((column) => (
                        <HeaderCell key={column.id} column={column} sort={sort} onSort={onSort} />
                    ))}
                </tr>
            </thead>

            <tbody>
                {state === "loading" ? <LoadingRows columns={columns.length} /> : null}

                {state === "empty" && empty !== undefined ? (
                    <tr>
                        <td colSpan={columns.length}>{empty}</td>
                    </tr>
                ) : null}

                {state === "loading" || state === "empty" ? null : (
                    <DataRows columns={columns} rows={rows} rowId={rowId} />
                )}
            </tbody>
        </table>
    );
}
