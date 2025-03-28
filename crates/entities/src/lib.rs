/// U+FFFD REPLACEMENT CHARACTER
const UNICODE_REPLACEMENT_CHAR: &[u8] = b"\xEF\xBF\xBD";

#[derive(Debug, PartialEq)]
pub enum HtmlContext {
    Attribute,
    BodyText,
    ForeignText,
    Script,
    Style,
}

pub fn decode(ctx: &HtmlContext, input: &[u8]) -> Box<[u8]> {
    let mut decoded: Vec<u8> = Vec::new();
    let end = input.len();
    let mut at = 0;
    let mut was_at = 0;

    while at + 3 < end {
        let next_character_reference_at = if let Some(pos) = memchr::memchr(b'&', &input[at..]) {
            at + pos
        } else {
            break;
        };

        if let Some((character_reference, token_len)) =
            decode_html_ref(ctx, input, next_character_reference_at)
        {
            // Do ambiguous checking for attributes.
            if *ctx == HtmlContext::Attribute {
                let is_ambiguous_entity_terminator =
                    input[next_character_reference_at + token_len - 1] != b';';

                // Ambiguous entities are not terminated by a semicolon _and_ have trailing
                // characters that are alphanumeric or "=".
                if is_ambiguous_entity_terminator
                    && (end > next_character_reference_at + token_len
                        && (input[next_character_reference_at + token_len].is_ascii_alphanumeric()
                            || input[next_character_reference_at + token_len] == b'='))
                {
                    // @todo Can't this skip ahead to next_character_reference_at + 1?
                    at += 1;
                    continue;
                }
            }

            at = next_character_reference_at;
            decoded.extend_from_slice(&input[was_at..at]);
            decoded.extend_from_slice(&character_reference);
            at += token_len;
            was_at = at;
            continue;
        }

        // @todo Can't this skip ahead to next_character_reference_at + 1?
        at += 1;
    }

    if was_at < end {
        decoded.extend_from_slice(&input[was_at..]);
    }

    decoded.into_boxed_slice()
}

/// Decodes a reference to an HTML entity.
/// @todo Ambiguous entitites based on ctx?
pub fn decode_html_ref(
    ctx: &HtmlContext,
    input: &[u8],
    offset: usize,
) -> Option<(Box<[u8]>, usize)> {
    if input.len() < offset + 3 {
        return None;
    }

    if input[offset] != b'&' {
        return None;
    }

    if input[offset + 1] == b'#' {
        return decode_html5_numeric_character_reference(input, offset);
    }

    let prefix = [input[offset + 1], input[offset + 2]];
    gen_entities::entities_lookup!("crates/entities/data/entities.json");

    let candidates = ENTITIES.get(&prefix)?;
    candidates
        .iter()
        .find_map(|(suffix, decoded_bytes)| -> Option<(Box<[u8]>, usize)> {
            let len = suffix.len();
            if offset + 3 + len > input.len() {
                None
            } else {
                let candidate = &input[offset + 3..offset + 3 + len];
                if candidate == *suffix {
                    Some(((*decoded_bytes).into(), 3 + len))
                } else {
                    None
                }
            }
        })
}

fn decode_html5_numeric_character_reference(
    input: &[u8],
    offset: usize,
) -> Option<(Box<[u8]>, usize)> {
    static HEX_DIGITS: [u8; 256] = [
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
        255, 255, 255, 255, 255, 255, 255, 10, 11, 12, 13, 14, 15, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 10, 11, 12, 13, 14, 15, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    ];

    static CP1252_REPLACEMENTS: [u32; 32] = [
        0x20AC, // 0x80 -> EURO SIGN (€).
        0x81,   // 0x81 -> (no change).
        0x201A, // 0x82 -> SINGLE LOW-9 QUOTATION MARK (‚).
        0x0192, // 0x83 -> LATIN SMALL LETTER F WITH HOOK (ƒ).
        0x201E, // 0x84 -> DOUBLE LOW-9 QUOTATION MARK („).
        0x2026, // 0x85 -> HORIZONTAL ELLIPSIS (…).
        0x2020, // 0x86 -> DAGGER (†).
        0x2021, // 0x87 -> DOUBLE DAGGER (‡).
        0x02C6, // 0x88 -> MODIFIER LETTER CIRCUMFLEX ACCENT (ˆ).
        0x2030, // 0x89 -> PER MILLE SIGN (‰).
        0x0160, // 0x8A -> LATIN CAPITAL LETTER S WITH CARON (Š).
        0x2039, // 0x8B -> SINGLE LEFT-POINTING ANGLE QUOTATION MARK (‹).
        0x0152, // 0x8C -> LATIN CAPITAL LIGATURE OE (Œ).
        0x8D,   // 0x8D -> (no change).
        0x017D, // 0x8E -> LATIN CAPITAL LETTER Z WITH CARON (Ž).
        0x8F,   // 0x8F -> (no change).
        0x90,   // 0x90 -> (no change).
        0x2018, // 0x91 -> LEFT SINGLE QUOTATION MARK (‘).
        0x2019, // 0x92 -> RIGHT SINGLE QUOTATION MARK (’).
        0x201C, // 0x93 -> LEFT DOUBLE QUOTATION MARK (“).
        0x201D, // 0x94 -> RIGHT DOUBLE QUOTATION MARK (”).
        0x2022, // 0x95 -> BULLET (•).
        0x2013, // 0x96 -> EN DASH (–).
        0x2014, // 0x97 -> EM DASH (—).
        0x02DC, // 0x98 -> SMALL TILDE (˜).
        0x2122, // 0x99 -> TRADE MARK SIGN (™).
        0x0161, // 0x9A -> LATIN SMALL LETTER S WITH CARON (š).
        0x203A, // 0x9B -> SINGLE RIGHT-POINTING ANGLE QUOTATION MARK (›).
        0x0153, // 0x9C -> LATIN SMALL LIGATURE OE (œ).
        0x9D,   // 0x9D -> (no change).
        0x017E, // 0x9E -> LATIN SMALL LETTER Z WITH CARON (ž).
        0x0178, // 0x9F -> LATIN CAPITAL LETTER Y WITH DIAERESIS (Ÿ).
    ];

    let end = input.len();
    let mut at = offset;

    if end < offset + 3 {
        return None;
    }

    if input[at] != b'&' {
        return None;
    }

    if input[at + 1] != b'#' {
        return None;
    }

    at += 2;

    #[derive(PartialEq)]
    enum Base {
        Decimal,
        Hexadecimal,
    }

    let base = if b'X' == (input[at] & 0xDF) {
        at += 1;
        Base::Hexadecimal
    } else {
        Base::Decimal
    };

    let zeros_at = at;

    // Skip past all the zeros: in most cases there will be none.
    while at < end && b'0' == input[at] {
        at += 1;
    }
    let zero_count = at - zeros_at;

    let digits_at = at;
    if base == Base::Hexadecimal {
        while at < end && HEX_DIGITS[input[at] as usize] <= 0xF {
            at += 1;
        }
    } else {
        while at < end && HEX_DIGITS[input[at] as usize] <= 0x9 {
            at += 1;
        }
    }
    let digit_count = at - digits_at;
    let after_digits = at;

    let has_trailing_semicolon = (after_digits < end) && b';' == input[at];
    let end_of_span = if has_trailing_semicolon {
        after_digits + 1
    } else {
        after_digits
    };
    let matched_byte_length = end_of_span - offset;

    // `&#` or `&#x` without digits returns into plaintext.
    if zero_count == 0 && digit_count == 0 {
        return None;
    }

    // Whereas `&#` and only zeros is invalid.
    if digit_count == 0 {
        return Some((UNICODE_REPLACEMENT_CHAR.into(), matched_byte_length));
    }

    // If there are too many digits then it's not worth parsing. It's invalid.

    if digit_count > if base == Base::Hexadecimal { 6 } else { 7 } {
        return Some((UNICODE_REPLACEMENT_CHAR.into(), matched_byte_length));
    }

    let mut code_point = 0u32;
    at = digits_at;
    if base == Base::Hexadecimal {
        for _ in 0..digit_count {
            code_point <<= 4;
            code_point += HEX_DIGITS[input[at] as usize] as u32;
            at += 1;
        }
    } else {
        for _ in 0..digit_count {
            code_point *= 10;
            code_point += HEX_DIGITS[input[at] as usize] as u32;
            at += 1;
        }
    }

    if (0x80..=0x9F).contains(&code_point) {
        code_point = CP1252_REPLACEMENTS[(code_point - 0x80) as usize];
    }

    if (0xD800..=0xDFFF).contains(&code_point) {
        return Some((UNICODE_REPLACEMENT_CHAR.into(), matched_byte_length));
    }

    /* Should these be disallowed here???

    // A noncharacter is a code point that is in the range U+FDD0 to U+FDEF, inclusive,
    // or U+FFFE, U+FFFF, U+1FFFE, U+1FFFF, U+2FFFE, U+2FFFF, U+3FFFE, U+3FFFF,
    // U+4FFFE, U+4FFFF, U+5FFFE, U+5FFFF, U+6FFFE, U+6FFFF, U+7FFFE, U+7FFFF,
    // U+8FFFE, U+8FFFF, U+9FFFE, U+9FFFF, U+AFFFE, U+AFFFF, U+BFFFE, U+BFFFF,
    // U+CFFFE, U+CFFFF, U+DFFFE, U+DFFFF, U+EFFFE, U+EFFFF, U+FFFFE, U+FFFFF,
    // U+10FFFE, or U+10FFFF.
    if matches!(
        code_point,
        0xFDD0
            ..=0xFDEF
                | 0xFFFE
                | 0xFFFF
                | 0x1FFFE
                | 0x1FFFF
                | 0x2FFFE
                | 0x2FFFF
                | 0x3FFFE
                | 0x3FFFF
                | 0x4FFFE
                | 0x4FFFF
                | 0x5FFFE
                | 0x5FFFF
                | 0x6FFFE
                | 0x6FFFF
                | 0x7FFFE
                | 0x7FFFF
                | 0x8FFFE
                | 0x8FFFF
                | 0x9FFFE
                | 0x9FFFF
                | 0xAFFFE
                | 0xAFFFF
                | 0xBFFFE
                | 0xBFFFF
                | 0xCFFFE
                | 0xCFFFF
                | 0xDFFFE
                | 0xDFFFF
                | 0xEFFFE
                | 0xEFFFF
                | 0xFFFFE
                | 0xFFFFF
                | 0x10FFFE
                | 0x10FFFF
    ) {
        return Some((UNICODE_REPLACEMENT_CHAR.into(), matched_byte_length));
    }

    // > A control is a C0 control or a code point in the range
    // > U+007F DELETE to U+009F APPLICATION PROGRAM COMMAND, inclusive.
    if matches!(code_point, 0x007F..=0x009F) {
        return Some((UNICODE_REPLACEMENT_CHAR.into(), matched_byte_length));
    }

    */

    Some((
        html5_code_point_to_utf8_bytes(code_point),
        matched_byte_length,
    ))
}

fn html5_code_point_to_utf8_bytes(code_point: u32) -> Box<[u8]> {
    let mut slice = [0u8; 4];
    char::from_u32(code_point).map_or(UNICODE_REPLACEMENT_CHAR.into(), |c| {
        c.encode_utf8(&mut slice);
        slice[..c.len_utf8()].into()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_out_of_range_numeric_hex_entity() {
        let input = b"&#xFFFFFF;";
        let decoded = decode(&HtmlContext::BodyText, input);
        let decoded = String::from_utf8(decoded.to_vec()).unwrap();
        assert_eq!(decoded, "\u{FFFD}");
    }

    #[test]
    fn decode_ref_out_of_range_numeric_hex_entity() {
        let input = b"&#xFFFFFF;";
        let (decoded, token_len) = decode_html_ref(&HtmlContext::BodyText, input, 0).unwrap();
        let decoded = String::from_utf8(decoded.to_vec()).unwrap();
        assert_eq!(decoded, "\u{FFFD}");
        assert_eq!(token_len, 10);
    }

    #[test]
    fn test_decode_html() {
        let input = b"&LT";
        let (decoded, len) = decode_html_ref(&HtmlContext::BodyText, input, 0).unwrap();
        assert_eq!(decoded, b"<".as_slice().into());
        assert_eq!(len, 3);
    }

    #[test]
    fn test_aelig_entity() {
        let (decoded, token_len) = decode_html_ref(&HtmlContext::BodyText, b"&AElig;", 0).unwrap();
        let decoded = String::from_utf8_lossy(&decoded);
        assert_eq!(decoded, "Æ");
        assert_eq!(token_len, 7);
    }

    #[test]
    fn test_named_entities() {
        // Common named entities
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&amp;", 0),
            Some((b"&".as_slice().into(), 5))
        );
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&lt;", 0),
            Some((b"<".as_slice().into(), 4))
        );
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&gt;", 0),
            Some((b">".as_slice().into(), 4))
        );
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&quot;", 0),
            Some((b"\"".as_slice().into(), 6))
        );

        // Case sensitivity
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&LT;", 0),
            Some((b"<".as_slice().into(), 4))
        );

        // With and without trailing semicolon
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&nbsp", 0),
            Some((b"\xC2\xA0".as_slice().into(), 5))
        );
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&nbsp;", 0),
            Some((b"\xC2\xA0".as_slice().into(), 6))
        );
    }

    #[test]
    fn test_numeric_decimal_entities() {
        // ASCII range
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&#65;", 0),
            Some((b"A".as_slice().into(), 5))
        );

        // Multi-byte UTF-8
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&#8364;", 0),
            Some((b"\xE2\x82\xAC".as_slice().into(), 7)) // Euro sign
        );

        // Without semicolon
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&#65", 0),
            Some((b"A".as_slice().into(), 4))
        );

        // With leading zeros
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&#0065;", 0),
            Some((b"A".as_slice().into(), 7))
        );
    }

    #[test]
    fn test_numeric_hex_entities() {
        // ASCII with 'x'
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&#x41;", 0),
            Some((b"A".as_slice().into(), 6))
        );

        // ASCII with 'X'
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&#X41;", 0),
            Some((b"A".as_slice().into(), 6))
        );

        // Multi-byte UTF-8
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&#x20AC;", 0),
            Some((b"\xE2\x82\xAC".as_slice().into(), 8)) // Euro sign
        );

        // Without semicolon
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&#x41", 0),
            Some((b"A".as_slice().into(), 5))
        );

        // With leading zeros
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&#x0041;", 0),
            Some((b"A".as_slice().into(), 8))
        );
    }

    #[test]
    fn test_cp1252_replacements() {
        // Test CP1252 replacement for code point 0x80 (EURO SIGN)
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&#128;", 0),
            Some((b"\xE2\x82\xAC".as_slice().into(), 6)) // Euro sign
        );

        // Test CP1252 replacement for code point 0x82 (SINGLE LOW-9 QUOTATION MARK)
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&#130;", 0),
            Some((b"\xE2\x80\x9A".as_slice().into(), 6))
        );
    }

    #[test]
    fn test_invalid_entities() {
        // Invalid surrogates
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&#xD800;", 0),
            Some((UNICODE_REPLACEMENT_CHAR.into(), 8))
        );

        // Just "&#" without digits
        assert_eq!(decode_html_ref(&HtmlContext::BodyText, b"&#;", 0), None);

        // "&#" with only zeros
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&#0;", 0),
            Some((UNICODE_REPLACEMENT_CHAR.into(), 4))
        );

        // Too many digits for hex
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&#x1234567;", 0),
            Some((UNICODE_REPLACEMENT_CHAR.into(), 11))
        );

        // Too many digits for decimal
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&#12345678;", 0),
            Some((UNICODE_REPLACEMENT_CHAR.into(), 11))
        );
    }

    #[test]
    fn test_entity_with_offset() {
        let input = b"text&amp;more";
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, input, 4),
            Some((b"&".as_slice().into(), 5))
        );

        // Offset too large
        assert_eq!(decode_html_ref(&HtmlContext::BodyText, input, 10), None);
    }

    #[test]
    fn test_non_entity_input() {
        // No & character
        assert_eq!(decode_html_ref(&HtmlContext::BodyText, b"text", 0), None);

        // Input too short
        assert_eq!(decode_html_ref(&HtmlContext::BodyText, b"&", 0), None);
        assert_eq!(decode_html_ref(&HtmlContext::BodyText, b"&;", 0), None);
        assert_eq!(decode_html_ref(&HtmlContext::BodyText, b"&A;", 0), None);
        assert_eq!(decode_html_ref(&HtmlContext::BodyText, b"&AE;", 0), None);
    }

    #[test]
    fn test_php_reference_cases() {
        // Test cases from PHP's decode_html_ref_1.phpt file
        // https://github.com/php/php-src/blob/93844af94e3cdeb9cdd457ec72d91554b44b7fba/ext/standard/tests/strings/decode_html_ref_1.phpt

        // "Test &#38;" -> "&"
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&#38;", 0),
            Some((b"&".as_slice().into(), 5))
        );

        // "Test &#x26;" -> "&"
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&#x26;", 0),
            Some((b"&".as_slice().into(), 6))
        );

        // "Test &#X26;" -> "&"
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&#X26;", 0),
            Some((b"&".as_slice().into(), 6))
        );

        // "Test &amp;" -> "&"
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&amp;", 0),
            Some((b"&".as_slice().into(), 5))
        );

        // "Test &#0038;" -> "&"
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&#0038;", 0),
            Some((b"&".as_slice().into(), 7))
        );

        // "Test &#x0026;" -> "&"
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&#x0026;", 0),
            Some((b"&".as_slice().into(), 8))
        );

        // "Test &notanentity;" -> None (original test returns string unchanged)
        let (decoded, token_len) =
            decode_html_ref(&HtmlContext::BodyText, b"&notanentity;", 0).unwrap();
        let decoded =
            String::from_utf8(decoded.to_vec()).expect("decoded string must be valid utf-8 bytes.");
        assert_eq!(decoded, "¬");
        assert_eq!(token_len, 4);

        // "Test &#38 xxx" -> "&" (entity without semicolon)
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&#38 xxx", 0),
            Some((b"&".as_slice().into(), 4))
        );

        // "Test &#x26 xxx" -> "&" (hex entity without semicolon)
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&#x26 xxx", 0),
            Some((b"&".as_slice().into(), 5))
        );

        // "Test &amp xxx" -> "&" (named entity without semicolon)
        assert_eq!(
            decode_html_ref(&HtmlContext::BodyText, b"&amp xxx", 0),
            Some((b"&".as_slice().into(), 4))
        );

        // "Simultaneously testing numeric (&#38;) and named (&amp;) entities" -> check portions
        let input = b"Simultaneously testing numeric (&#0038;) and named (&amp;) entities";

        // Testing numeric entity starting at offset 32
        let (decoded, token_len) = decode_html_ref(&HtmlContext::BodyText, input, 32).unwrap();
        let decoded = String::from_utf8_lossy(&decoded);
        assert_eq!(decoded, "&");
        assert_eq!(token_len, 7);

        // Testing named entity starting at offset 52
        let (decoded, token_len) = decode_html_ref(&HtmlContext::BodyText, input, 52).unwrap();
        let decoded = String::from_utf8_lossy(&decoded);
        assert_eq!(decoded, "&");
        assert_eq!(token_len, 5);

        // "Test &;" -> None (entity with no name)
        assert_eq!(decode_html_ref(&HtmlContext::BodyText, b"&;", 0), None);
    }
}
