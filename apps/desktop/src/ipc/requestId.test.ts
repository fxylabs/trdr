import { expect, test } from "vitest";

import { REQUEST_ID_LENGTH, newRequestId } from "./requestId";

/**
 * The rules `trdr_core::id::parse_ulid` applies, restated here.
 *
 * Twenty-six characters of Crockford base32, and a first character of `0` to
 * `7`. The last rule is the one worth spelling out: the encoding has room for
 * 130 bits and a ULID is 128, so `8` through `Z` in the leading position name a
 * value that does not fit, and Rust refuses it rather than truncating. An id
 * this file produced that failed it would be refused at the IPC boundary, which
 * is the failure this test exists to make impossible.
 */
const ACCEPTED_BY_THE_HOST = /^[0-7][0-9A-HJKMNP-TV-Z]{25}$/;

test("an id is the shape the host parses", () =>
{
    const id = newRequestId();

    expect(id).toHaveLength(REQUEST_ID_LENGTH);
    expect(id).toMatch(ACCEPTED_BY_THE_HOST);
});

/**
 * Not a fixed sample: the leading character depends on the clock, and the check
 * that matters is that no clock this side can produce makes an id Rust refuses.
 */
test("no instant a clock can name produces an id the host would refuse", () =>
{
    const instants = [
        0,
        1,
        Date.now(),
        // The last millisecond 48 bits can hold, and past it.
        2 ** 48 - 1,
        2 ** 48,
        2 ** 53,
        // Values a caller should never pass, which must still not produce
        // something malformed.
        -1,
        Number.NaN,
        Number.POSITIVE_INFINITY
    ];

    for (const instant of instants)
    {
        expect(newRequestId(instant), `at ${instant}`).toMatch(ACCEPTED_BY_THE_HOST);
    }
});

test("the timestamp is the leading ten characters and sorts by time", () =>
{
    const earlier = newRequestId(1_786_356_000_000);
    const later = newRequestId(1_786_356_000_001);

    expect(earlier.slice(0, 10) < later.slice(0, 10)).toBe(true);
    expect(newRequestId(1_786_356_000_000).slice(0, 10)).toBe(earlier.slice(0, 10));
});

/**
 * The whole point of the random tail. Two commands issued in one tick — which
 * is what mounting the shell does — must not correlate to each other.
 */
test("two ids minted in the same millisecond are different", () =>
{
    const now = 1_786_356_000_000;
    const minted = new Set(Array.from({ length: 2_000 }, () => newRequestId(now)));

    expect(minted.size).toBe(2_000);
});

/**
 * A generator that leans on one part of the alphabet is a generator whose
 * randomness is narrower than it looks. Eighty bits over 2 000 draws touches
 * every one of the 32 characters with overwhelming probability.
 */
test("the random tail uses the whole alphabet", () =>
{
    const seen = new Set<string>();

    for (let draw = 0; draw < 2_000; draw += 1)
    {
        for (const character of newRequestId().slice(10))
        {
            seen.add(character);
        }
    }

    expect(seen.size).toBe(32);
    expect([...seen].some((character) => "ILOU".includes(character))).toBe(false);
});
