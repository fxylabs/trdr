/**
 * Moving raw bytes across the boundary without turning them into text.
 *
 * Terminal traffic is not text and cannot be treated as text anywhere on the
 * way through. A read on the host side can end in the middle of a multi-byte
 * character, an agent painting its screen emits bytes that are not characters
 * at all, and a `TextDecoder` would replace every one of those with U+FFFD. A
 * replaced byte is a corrupted stream.
 *
 * So the wire carries base64 — `trdr_core::ui::TerminalInputParams` fixes that
 * for keystrokes and `TerminalOutput` mirrors it for output — and this file is
 * the only place either direction is converted.
 *
 * # The split sequence, and who reassembles it
 *
 * Nothing here does. A chunk that ends halfway through a character arrives as
 * the bytes it is, and its other half arrives in the next chunk. xterm.js is
 * what puts them back together: writing a `Uint8Array` to a terminal goes
 * through its own incremental UTF-8 decoder, which keeps a partial sequence
 * between writes. Writing a *string* would not — the split would already have
 * happened by then — which is why `XtermHost` hands it bytes.
 */

/** The bytes this base64 text stands for. */
export function decodeBase64(text: string): Uint8Array
{
    const binary = atob(text);
    const bytes = new Uint8Array(binary.length);

    for (let index = 0; index < binary.length; index += 1)
    {
        bytes[index] = binary.charCodeAt(index);
    }

    return bytes;
}

/**
 * These bytes, as base64.
 *
 * Written one character at a time rather than with `String.fromCharCode(...bytes)`,
 * which spreads the whole array into an argument list and overflows the stack
 * somewhere above a hundred thousand bytes. Nothing sent from a keyboard is
 * that large, and a limit that only fails for the largest paste anyone ever
 * makes is the kind that is found by a user.
 */
export function encodeBase64(bytes: Uint8Array): string
{
    let binary = "";

    for (const byte of bytes)
    {
        binary += String.fromCharCode(byte);
    }

    return btoa(binary);
}

/**
 * What xterm's `onData` gives, as the bytes a terminal would have sent.
 *
 * `onData` hands over a JavaScript string, and a JavaScript string is UTF-16.
 * `btoa` on one throws the moment a character is above U+00FF, so a person
 * typing Korean into the agent would have hit an exception rather than sent a
 * character. Encoding to UTF-8 first is what a real terminal does with the same
 * keystroke.
 */
export function typedBytes(text: string): Uint8Array
{
    return new TextEncoder().encode(text);
}

/**
 * What xterm's `onBinary` gives, as bytes.
 *
 * A different string from `onData`'s: `onBinary` carries bytes already, one per
 * code unit, and encoding it as UTF-8 would turn every byte above 0x7f into
 * two.
 */
export function binaryBytes(text: string): Uint8Array
{
    const bytes = new Uint8Array(text.length);

    for (let index = 0; index < text.length; index += 1)
    {
        bytes[index] = text.charCodeAt(index) & 0xff;
    }

    return bytes;
}
