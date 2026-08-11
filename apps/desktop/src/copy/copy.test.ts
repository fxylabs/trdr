import { expect, test } from "vitest";

import {
    BROKER_CONNECTION,
    ERROR,
    MARKET_TONE,
    PAPER_VALIDATION,
    SECTION_STATE,
    SUPPORT,
    TODAY_EVENT_KIND,
    VERDICT,
    warningSentence,
    words
} from "./ko";
import { decimal, instant, percent, share, shares, signedPercent, signedWon, won } from "./format";

/**
 * Section 12's list is closed, so the table over it can be complete — and this
 * is the check that it is. `Record<ErrorCode, string>` already makes a missing
 * key a compile error; what a type cannot catch is a key that was added with an
 * empty string or with the code copied into it, which is the shape a hurried
 * fill-in takes.
 */
test("every error code reaches a person as a sentence rather than as a code", () =>
{
    for (const [code, sentence] of Object.entries(ERROR))
    {
        expect(sentence.length, code).toBeGreaterThan(4);
        expect(sentence, code).not.toContain(code);
        expect(sentence.endsWith("."), code).toBe(true);
    }

    expect(Object.keys(ERROR)).toHaveLength(35);
});

/** Every finite state a screen can receive has words. Empty is not a word. */
test("every state model maps to something a person can read", () =>
{
    const tables = [
        SECTION_STATE,
        BROKER_CONNECTION,
        PAPER_VALIDATION,
        SUPPORT,
        VERDICT,
        MARKET_TONE,
        TODAY_EVENT_KIND
    ];

    for (const table of tables)
    {
        for (const [state, label] of Object.entries(table))
        {
            expect(label.length, state).toBeGreaterThan(0);
            expect(label, state).not.toBe(state);
        }
    }
});

/** A code with no wording yet is shown as itself, which is reportable. */
test("an unknown code falls back to the code rather than to nothing", () =>
{
    expect(words("observation-day-recorded")).toBe("관측일 하나가 기록되었습니다.");
    expect(words("something-nobody-has-written-yet")).toBe("something-nobody-has-written-yet");
});

/** A warning is a code and safe parameters, and the words are written here. */
test("a backtest warning becomes a sentence with its parameters beside it", () =>
{
    const sentence = warningSentence("DATA_INCOMPLETE", ["2026-06-15", "2026-06-17"]);

    expect(sentence).toContain(ERROR.DATA_INCOMPLETE);
    expect(sentence).toContain("2026-06-15 ~ 2026-06-17");
    expect(warningSentence("DATA_INCOMPLETE", [])).toBe(ERROR.DATA_INCOMPLETE);
});

/**
 * The rule section 7.2 puts on ratios, held at the last place it could be
 * broken. `4.12` is the digits of `0.0412` with the point moved, not
 * `0.0412 * 100` rounded — the second would be `4.119999999999999` before
 * rounding, and rounding is a decision this code has no standing to make.
 */
test("a ratio becomes a percentage by moving its point and nothing else", () =>
{
    expect(percent("0.0412")).toBe("4.12%");
    expect(percent("0.1030")).toBe("10.30%");
    expect(percent("0.0870")).toBe("8.70%");
    expect(percent("0.5710")).toBe("57.10%");
    expect(percent("0.0000")).toBe("0.00%");
    expect(percent("1.0000")).toBe("100.00%");
});

test("a change carries its sign, and a loss carries a minus rather than a hyphen", () =>
{
    expect(signedPercent("0.0180")).toBe("+1.80%");
    expect(signedPercent("-0.0240")).toBe("−2.40%");
    expect(signedWon(142_000)).toBe("+142,000원");
    expect(signedWon(-63_600)).toBe("−63,600원");
});

test("a ratio that is not a percentage keeps its value and drops its padding", () =>
{
    expect(decimal("0.6200")).toBe("0.62");
    expect(decimal("-1.2500")).toBe("−1.25");
    expect(decimal("2")).toBe("2");
});

/** A value that is not a decimal string is shown rather than turned into NaN. */
test("something that is not a ratio is left alone", () =>
{
    expect(percent("")).toBe("");
    expect(percent("n/a")).toBe("n/a");
    expect(decimal("n/a")).toBe("n/a");
});

test("money is grouped without going through a locale", () =>
{
    expect(won(12_480_000)).toBe("12,480,000원");
    expect(won(0)).toBe("0원");
    expect(won(-1)).toBe("−1원");
    expect(shares(320)).toBe("320주");
});

test("a share of a whole is clamped and never divides by zero", () =>
{
    expect(share(63, 68)).toBe(93);
    expect(share(0, 0)).toBe(0);
    expect(share(9, 60)).toBe(15);
    expect(share(70, 60)).toBe(100);
});

/**
 * The market's clock rather than the machine's. Both are UTC instants here, and
 * both have to read as the Seoul time they belong to wherever this test runs.
 */
test("an instant is read in the market's own zone", () =>
{
    expect(instant("2026-08-10T15:30:00Z")).toContain("2026");
    expect(instant("2026-08-10T15:30:00Z")).toContain("00:30");
    expect(instant("not an instant")).toBe("not an instant");
});
