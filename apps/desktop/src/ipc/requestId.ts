/**
 * Minting the request id every command carries.
 *
 * # Why the WebView mints it and the host does not
 *
 * `UiCommandEnvelope` in `trdr-core` puts three fields on every command — a
 * protocol version, a request id, and the command itself — and the id is the
 * caller's. It has to be: it is what lets the side that is *waiting* name what
 * it is waiting for. A host-minted id arrives with the answer and is therefore
 * useless for anything the caller wanted it for — correlating a retry, matching
 * a log line to a click, naming the request in an approval sheet the screen
 * raised. Section 9.2 requires a repeated id to get the same terminal result
 * rather than a second execution, and only the caller knows which of its
 * requests is a repeat of which.
 *
 * The alternative was to drop the parameter and have the host mint one. It
 * would be less code here and would also delete the boundary this scaffold
 * exists to prove: `ping` takes a `RequestId`, not a string, so a WebView that
 * sends anything but 26 characters of Crockford base32 is refused by
 * deserialisation before the handler is entered. That refusal is what makes the
 * bridge typed rather than merely generated, and it needs a caller that supplies
 * an id to be a refusal of anything.
 *
 * # What this is not
 *
 * It is not a general ULID library, and nothing outside this file should treat
 * it as one. Monotonicity within a millisecond is not implemented, because
 * nothing here depends on two ids made in the same millisecond being ordered —
 * they only have to be different, and 80 bits of randomness is what makes them
 * so. If a later feature needs a sortable id, it belongs in Rust, where
 * `trdr-runtime` already mints them properly.
 */

/**
 * Crockford base32, which is the ULID alphabet.
 *
 * `I`, `L`, `O`, and `U` are missing on purpose: the first three cannot be
 * misread as `1` and `0`, and the fourth cannot appear inside an unfortunate
 * word. `trdr_core::id` parses exactly this alphabet.
 */
const ALPHABET = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/** How many characters carry the timestamp. 10 × 5 bits, holding 48. */
const TIME_CHARACTERS = 10;

/** How many carry the random tail. 16 × 5 bits, which is exactly 80. */
const RANDOM_CHARACTERS = 16;

/** The largest instant 48 bits can name, in milliseconds. */
const LAST_INSTANT = 2 ** 48 - 1;

/** How long every id is. `trdr_core::id::ID_LENGTH` says the same number. */
export const REQUEST_ID_LENGTH = TIME_CHARACTERS + RANDOM_CHARACTERS;

/**
 * A fresh request id.
 *
 * @param now - The instant to stamp it with, for a test that wants a known
 *   prefix. Callers in the app leave it out.
 */
export function newRequestId(now: number = Date.now()): string
{
    return encodeTime(now) + encodeRandom();
}

/**
 * The instant, as ten characters, most significant first.
 *
 * Division rather than bit shifting, because JavaScript's bitwise operators
 * truncate to 32 bits and a millisecond timestamp needs 41. Integer division of
 * a value below 2^53 is exact, so nothing is lost.
 */
function encodeTime(milliseconds: number): string
{
    let remaining = Number.isFinite(milliseconds)
        ? Math.min(Math.max(Math.floor(milliseconds), 0), LAST_INSTANT)
        : 0;
    let text = "";

    for (let written = 0; written < TIME_CHARACTERS; written += 1)
    {
        text = ALPHABET[remaining % 32] + text;
        remaining = Math.floor(remaining / 32);
    }

    return text;
}

/**
 * Eighty bits of randomness, as sixteen characters.
 *
 * The low five bits of each byte are taken. That is uniform without rejection
 * sampling because 256 is a whole multiple of 32 — every one of the 32 values
 * comes from exactly eight of the 256 bytes. Taking `byte % 30`, or slicing a
 * value out of `Math.random`, would not be.
 */
function encodeRandom(): string
{
    const bytes = new Uint8Array(RANDOM_CHARACTERS);
    crypto.getRandomValues(bytes);

    let text = "";

    for (const byte of bytes)
    {
        text += ALPHABET[byte & 31];
    }

    return text;
}
