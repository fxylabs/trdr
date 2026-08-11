import { expect, test } from "vitest";

import { binaryBytes, decodeBase64, encodeBase64, typedBytes } from "./bytes";

/**
 * Byte arrays are compared as plain numbers.
 *
 * `TextEncoder` in this test environment is the document's, and its
 * `Uint8Array` is a different constructor from the one this module sees, so two
 * arrays holding identical bytes are not equal to a structural comparison. The
 * difference is an artefact of running a document inside Node and exists
 * nowhere in a WebView; the bytes are what the assertions are about either way.
 */
function bytesOf(bytes: Uint8Array): number[]
{
    return [...bytes];
}

/**
 * The property the whole byte path rests on. Escape bytes and values that are
 * not valid UTF-8 in any position are what a terminal is made of, and every one
 * of them has to survive the crossing as itself.
 */
test("every byte value survives the round trip", () =>
{
    const every = new Uint8Array(256).map((_, index) => index);

    expect(bytesOf(decodeBase64(encodeBase64(every)))).toEqual(bytesOf(every));
});

/** The vectors from RFC 4648, so this is the same base64 the host writes. */
test("the encoding is the one the host and the standard agree on", () =>
{
    const pairs: readonly (readonly [string, string])[] = [
        ["", ""],
        ["f", "Zg=="],
        ["fo", "Zm8="],
        ["foo", "Zm9v"],
        ["foobar", "Zm9vYmFy"]
    ];

    for (const [text, base64] of pairs)
    {
        expect(encodeBase64(typedBytes(text))).toBe(base64);
        expect(bytesOf(decodeBase64(base64))).toEqual(bytesOf(typedBytes(text)));
    }
});

/**
 * A read on the host side can end in the middle of a character. Both halves are
 * carried as the bytes they are, and putting them back together is the
 * terminal's job — which it can only do if nothing lost a byte first.
 */
test("a character split across two chunks arrives as its two halves", () =>
{
    const whole = typedBytes("한");
    const halves = [whole.slice(0, 1), whole.slice(1)];

    const rebuilt = halves.flatMap((half) => bytesOf(decodeBase64(encodeBase64(half))));

    expect(rebuilt).toEqual(bytesOf(whole));
});

/**
 * The failure this rules out: `btoa` throws on any character above U+00FF, so
 * a rail that encoded the typed string directly would raise an exception the
 * first time someone typed Korean at the agent.
 */
test("what is typed is sent as the bytes a terminal would have sent", () =>
{
    expect(() => encodeBase64(typedBytes("전략"))).not.toThrow();
    expect(bytesOf(typedBytes("전략"))).toEqual([0xec, 0xa0, 0x84, 0xeb, 0x9e, 0xb5]);
    expect(bytesOf(typedBytes("\r"))).toEqual([0x0d]);
});

/**
 * xterm's binary channel carries bytes already, one per code unit. Encoding it
 * as UTF-8 would turn every byte above 0x7f into two.
 */
test("binary input is one byte per code unit", () =>
{
    expect(bytesOf(binaryBytes("\u001b\u00ff"))).toEqual([0x1b, 0xff]);
    expect(bytesOf(typedBytes("ÿ"))).toEqual([0xc3, 0xbf]);
});
