/**
 * Turning a model's values into the characters a person reads.
 *
 * Formatting is presentation, so it belongs on this side of the bridge rather
 * than in a query model. Two rules from section 7.2 survive the crossing and
 * shape everything below.
 *
 * # A ratio is a decimal string and stays one
 *
 * `Ratio` is `"0.0412"` — a string at a fixed scale of four places, because the
 * same value goes into the hashes that make a backtest reproducible and a
 * binary float is not reproducible across the machines that would have to
 * agree. Reading one into a JavaScript number to multiply it by a hundred and
 * round it would undo that at the last possible moment, and would do it
 * invisibly. So [`percent`] moves the decimal point two places by rewriting the
 * string, and no function here calls `Number` on a ratio.
 *
 * # Money is an integer of won
 *
 * `Krw` is a whole number and is grouped by inserting separators into its
 * digits. `toLocaleString` is not used: its output depends on the runtime's
 * locale data, and a value that renders differently on two machines is a value
 * a screenshot cannot be checked against. What it produces would also be a
 * locale-formatted string, which must never become the input to anything.
 *
 * # A time is shown in the market's zone, not the machine's
 *
 * Instants arrive as UTC. Converting with the machine's own zone would make the
 * same fixture read differently on two Macs, and a test would pass wherever it
 * was written. `Asia/Seoul` is named explicitly for the market these numbers
 * belong to.
 */

/** Separators every three digits, without going through a locale. */
function group(digits: string): string
{
    return digits.replace(/\B(?=(\d{3})+(?!\d))/g, ",");
}

/** U+2212, which is what the gallery draws and what a minus sign is. */
const MINUS = "−";

/** An amount of won, grouped, with its unit. */
export function won(amount: number): string
{
    const sign = amount < 0 ? MINUS : "";

    return `${sign}${group(String(Math.abs(amount)))}원`;
}

/** The same, with the sign always present because the amount is a change. */
export function signedWon(amount: number): string
{
    const sign = amount < 0 ? MINUS : "+";

    return `${sign}${group(String(Math.abs(amount)))}원`;
}

/** A whole number of things, grouped. */
export function count(value: number): string
{
    return group(String(value));
}

/** How a decimal string splits, or nothing when it is not one. */
function splitDecimal(value: string): { negative: boolean; digits: string; point: number } | undefined
{
    const match = /^(-?)(\d+)(?:\.(\d+))?$/.exec(value);

    if (match === null)
    {
        return undefined;
    }

    const whole = match[2] ?? "";
    const fraction = match[3] ?? "";

    return { negative: match[1] === "-", digits: whole + fraction, point: whole.length };
}

/** The digits with the point moved right, as `integer.fraction`. */
function movePoint(digits: string, point: number, places: number): string
{
    const padded = digits.padEnd(point + places, "0");
    const at = point + places;
    const whole = padded.slice(0, at).replace(/^0+(?=\d)/, "");
    const fraction = padded.slice(at);

    return fraction === "" ? whole : `${whole}.${fraction}`;
}

/**
 * A ratio as a percentage, by moving its decimal point and nothing else.
 *
 * `"0.0412"` becomes `"4.12%"` and `"0.1030"` becomes `"10.30%"`. The trailing
 * zero is kept on purpose: it is the scale the value was written at, and
 * trimming it would claim a precision decision this function has no standing to
 * make. A string that is not a decimal is returned untouched rather than turned
 * into `NaN%`.
 */
export function percent(ratio: string): string
{
    const parts = splitDecimal(ratio);

    if (parts === undefined)
    {
        return ratio;
    }

    return `${parts.negative ? MINUS : ""}${movePoint(parts.digits, parts.point, 2)}%`;
}

/** The same, with the sign always present because the ratio is a change. */
export function signedPercent(ratio: string): string
{
    const parts = splitDecimal(ratio);

    if (parts === undefined)
    {
        return ratio;
    }

    return `${parts.negative ? MINUS : "+"}${movePoint(parts.digits, parts.point, 2)}%`;
}

/** A ratio that is not a percentage — a Sharpe, say — with its zeros trimmed. */
export function decimal(ratio: string): string
{
    const parts = splitDecimal(ratio);

    if (parts === undefined)
    {
        return ratio;
    }

    const trimmed = ratio.includes(".") ? ratio.replace(/0+$/, "").replace(/\.$/, "") : ratio;

    return parts.negative ? trimmed.replace("-", MINUS) : trimmed;
}

/**
 * A share of a whole, as a whole percent.
 *
 * Both arguments are counts of market days rather than a ratio, so arithmetic
 * on them is exact and the warning at the top of this file does not apply.
 */
export function share(part: number, whole: number): number
{
    if (whole <= 0)
    {
        return 0;
    }

    return Math.max(0, Math.min(100, Math.round((part / whole) * 100)));
}

/** How the market's own clock is read, wherever this window is running. */
const SEOUL =
{
    timeZone: "Asia/Seoul",
    hour12: false
} as const;

const DATE_TIME = new Intl.DateTimeFormat("ko-KR", {
    ...SEOUL,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit"
});

const TIME = new Intl.DateTimeFormat("ko-KR", { ...SEOUL, hour: "2-digit", minute: "2-digit" });

/** An instant as a date and a time, or its own text when it is not an instant. */
export function instant(value: string): string
{
    const at = new Date(value);

    return Number.isNaN(at.getTime()) ? value : DATE_TIME.format(at);
}

/** The same instant as a clock time alone, for a row that already has its date. */
export function clockTime(value: string): string
{
    const at = new Date(value);

    return Number.isNaN(at.getTime()) ? value : TIME.format(at);
}

/** A hash, shortened to the prefix a person compares by eye. */
export function shortHash(hash: string): string
{
    return hash.slice(0, 12);
}

/** A quantity of shares. */
export function shares(quantity: number): string
{
    return `${count(quantity)}주`;
}
