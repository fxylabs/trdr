//! The encoding the terminal's bytes cross the WebView boundary in.
//!
//! `trdr_core::ui::TerminalInputParams` already fixes base64 for keystrokes on
//! the way in, and the way out is its mirror. The reason is the same in both
//! directions and it is not a preference: terminal traffic is not text. A single
//! read can end in the middle of a multi-byte character, an agent's screen
//! painting is full of bytes that are not characters at all, and anything that
//! passed the stream through a UTF-8 conversion would replace what it could not
//! decode. A replaced byte is a corrupted stream, and the corruption shows up
//! later as a glyph nobody can explain.
//!
//! Base64 carries arbitrary bytes through a JSON string with nothing looking at
//! them, which is exactly what section 11's rule asks for.
//!
//! # Why this is written out rather than depended on
//!
//! The same argument [`crate::clock`] makes for its calendar arithmetic. This is
//! forty lines of a fixed, standard table with no configuration, no alphabet
//! choice, and no streaming mode, and every case it can produce is covered
//! below. A dependency here would be larger than the thing it replaced, and it
//! would sit on the one path where every byte of the agent's output travels.

/// The standard alphabet (RFC 4648 §4), which is what `atob` in a WebView reads.
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Six bits, for masking one character's worth out of the accumulator.
const SIX_BITS: u32 = 0x3f;

/// Four characters' worth, which is the most the accumulator ever holds.
const TWENTY_FOUR_BITS: u32 = 0x00ff_ffff;

/// These bytes, as base64 with padding.
pub fn encode(bytes: &[u8]) -> String
{
    let mut text = String::with_capacity(bytes.len().div_ceil(3) * 4);

    for group in bytes.chunks(3)
    {
        let packed = group
            .iter()
            .fold(0u32, |packed, byte| (packed << 8) | u32::from(*byte))
            << (8 * (3 - group.len()));

        for position in 0..4
        {
            match position <= group.len()
            {
                true => text.push(char::from(
                    ALPHABET[((packed >> (18 - 6 * position)) & SIX_BITS) as usize]
                )),
                false => text.push('=')
            }
        }
    }

    text
}

/// The bytes this base64 text stands for, or why it is not base64.
///
/// Strict: a length that is not a multiple of four, padding in the wrong place,
/// and any character outside the alphabet are all refused. The text comes from
/// the WebView, and a decoder that guessed at damaged input would be feeding
/// invented bytes to a process.
pub fn decode(text: &str) -> Result<Vec<u8>, Base64Error>
{
    if !text.len().is_multiple_of(4)
    {
        return Err(Base64Error::Length { found: text.len() });
    }

    let body = text.trim_end_matches('=');

    if text.len() - body.len() > 2
    {
        return Err(Base64Error::Padding);
    }

    let mut bytes = Vec::with_capacity(body.len() / 4 * 3);
    let mut packed = 0u32;
    let mut held = 0u32;

    for character in body.bytes()
    {
        packed = ((packed << 6) | u32::from(sextet(character)?)) & TWENTY_FOUR_BITS;
        held += 6;

        if held >= 8
        {
            held -= 8;
            bytes.push((packed >> held) as u8);
        }
    }

    Ok(bytes)
}

/// The six bits one character stands for.
fn sextet(character: u8) -> Result<u8, Base64Error>
{
    ALPHABET
        .iter()
        .position(|known| *known == character)
        .map(|index| index as u8)
        .ok_or(Base64Error::Character)
}

/// Text that was handed over as base64 and is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Base64Error
{
    /// The length is not a multiple of four.
    #[error("base64 is four characters per group, and this is {found} characters")]
    Length
    {
        /// How long the text was.
        found: usize
    },
    /// More than two padding characters, or padding somewhere other than the end.
    #[error("the padding is not one or two characters at the end")]
    Padding,
    /// A character outside the alphabet.
    #[error("a character outside the base64 alphabet")]
    Character
}

#[cfg(test)]
mod tests
{
    use super::*;

    /// The vectors from RFC 4648 §10, which is what makes this the encoding
    /// everything else means by base64 rather than one that merely round-trips
    /// with itself.
    #[test]
    fn the_published_vectors_encode_the_way_the_standard_says()
    {
        for (bytes, text) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy")
        ]
        {
            assert_eq!(encode(bytes.as_bytes()), text, "encoding {bytes:?}");
            assert_eq!(decode(text).unwrap(), bytes.as_bytes(), "decoding {text:?}");
        }
    }

    /// What the terminal actually sends: escape bytes, and byte values that are
    /// not valid UTF-8 in any position.
    #[test]
    fn every_byte_value_survives_the_round_trip()
    {
        let every: Vec<u8> = (0..=255u8).collect();

        assert_eq!(decode(&encode(&every)).unwrap(), every);
    }

    /// A multi-byte character cut in half by a read boundary is two halves of
    /// invalid UTF-8, and both halves have to arrive as themselves.
    #[test]
    fn a_split_multi_byte_character_crosses_as_its_two_halves()
    {
        let whole = "한".as_bytes().to_vec();
        let (head, tail) = whole.split_at(1);

        let rebuilt = [
            decode(&encode(head)).unwrap(),
            decode(&encode(tail)).unwrap()
        ]
        .concat();

        assert_eq!(rebuilt, whole);
        assert_eq!(String::from_utf8(rebuilt).unwrap(), "한");
    }

    #[test]
    fn the_last_two_characters_of_the_alphabet_are_reached()
    {
        assert_eq!(encode(&[0xfb, 0xff, 0xfe]), "+//+");
        assert_eq!(decode("+//+").unwrap(), vec![0xfb, 0xff, 0xfe]);
    }

    #[test]
    fn text_that_is_not_base64_is_refused_rather_than_guessed_at()
    {
        assert_eq!(decode("Zg="), Err(Base64Error::Length { found: 3 }));
        assert_eq!(decode("Z==="), Err(Base64Error::Padding));
        assert_eq!(decode("Zm9$"), Err(Base64Error::Character));
        assert_eq!(decode("Z=m9"), Err(Base64Error::Character));
        // Four bytes, and not four base64 characters — the length check counts
        // bytes, so the alphabet check is what has to catch this.
        assert_eq!(decode("한a"), Err(Base64Error::Character));
        assert_eq!(decode("한"), Err(Base64Error::Length { found: 3 }));
    }

    /// A chunk the size the reader actually hands over, so the arithmetic is
    /// exercised at the length it runs at rather than only at six bytes.
    #[test]
    fn a_full_read_chunk_round_trips()
    {
        let chunk: Vec<u8> = (0..4096u32).map(|index| (index % 251) as u8).collect();
        let text = encode(&chunk);

        assert_eq!(text.len(), 4096_usize.div_ceil(3) * 4);
        assert_eq!(decode(&text).unwrap(), chunk);
    }
}
