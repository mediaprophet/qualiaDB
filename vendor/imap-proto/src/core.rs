use nom::{self, IResult};

use std::str;

// list-wildcards = "%" / "*"
pub fn list_wildcards(c: u8) -> bool {
    c == b'%' || c == b'*'
}

// quoted-specials = DQUOTE / "\"
pub fn quoted_specials(c: u8) -> bool {
    c == b'"' || c == b'\\'
}

// resp-specials = "]"
pub fn resp_specials(c: u8) -> bool {
    c == b']'
}

// atom-specials = "(" / ")" / "{" / SP / CTL / list-wildcards / quoted-specials / resp-specials
pub fn atom_specials(c: u8) -> bool {
    c == b'('
        || c == b')'
        || c == b'{'
        || c == b' '
        || c < 32
        || list_wildcards(c)
        || quoted_specials(c)
        || resp_specials(c)
}

// ATOM-CHAR = <any CHAR except atom-specials>
pub fn atom_char(c: u8) -> bool {
    !atom_specials(c)
}

// nil = "NIL"
named_a!(pub nil<'a, &'a [u8]>, tag_no_case!("NIL"));

// ASTRING-CHAR = ATOM-CHAR / resp-specials
pub fn astring_char(c: u8) -> bool {
    atom_char(c) || resp_specials(c)
}

// QUOTED-CHAR = <any TEXT-CHAR except quoted-specials> / "\" quoted-specials
pub fn quoted_data<'a>(i: &'a [u8]) -> IResult<&'a [u8], &'a [u8]> {
    // Ideally this should use nom's `escaped` macro, but it suffers from broken
    // type inference unless compiled with the verbose-errors feature enabled.
    let mut escape = false;
    let mut len = 0;
    for c in i {
        if *c == b'"' && !escape {
            break;
        }
        len += 1;
        if *c == b'\\' && !escape {
            escape = true
        } else if escape {
            escape = false;
        }
    }
    Ok((&i[len..], &i[..len]))
}

// quoted = DQUOTE *QUOTED-CHAR DQUOTE
named_a!(pub quoted<'a, &'a [u8]>, delimited!(
    char!('"'),
    quoted_data,
    char!('"')
));

// quoted bytes as as utf8
named_a!(pub quoted_utf8<'a, &'a str>, map_res!(quoted, str::from_utf8));

// literal = "{" number "}" CRLF *CHAR8
//            ; Number represents the number of CHAR8s
named_a!(pub literal<'a, &'a [u8]>, do_parse!(
    tag!("{") >>
    len: number >>
    tag!("}") >>
    tag!("\r\n") >>
    data: take!(len) >>
    (data)
));

// string = quoted / literal
named_a!(pub string<'a, &'a [u8]>, alt!(quoted | literal));

// string bytes as as utf8
named_a!(pub string_utf8<'a, &'a str>, map_res!(string, str::from_utf8));

// nstring = string / nil
named_a!(pub nstring<'a, Option<&'a [u8]>>, alt!(
    map!(nil, |_| None) |
    map!(string, |s| Some(s))
));

// nstring bytes as utf8
named_a!(pub nstring_utf8<'a, Option<&'a str>>, alt!(
    map!(nil, |_| None) |
    map!(string_utf8, |s| Some(s))
));

// number          = 1*DIGIT
//                    ; Unsigned 32-bit integer
//                    ; (0 <= n < 4,294,967,296)
named!(pub number<u32>, flat_map!(nom::character::streaming::digit1, parse_to!(u32)));

// same as `number` but 64-bit
named!(pub number_64<u64>, flat_map!(nom::character::streaming::digit1, parse_to!(u64)));

// atom = 1*ATOM-CHAR
named_a!(pub atom<'a, &'a str>, map_res!(take_while1!(atom_char),
    str::from_utf8
));

// astring = 1*ASTRING-CHAR / string
named_a!(pub astring<'a, &'a [u8]>, alt!(
    take_while1!(astring_char) |
    string
));

// astring bytes as as utf8
named_a!(pub astring_utf8<'a, &'a str>, map_res!(astring, str::from_utf8));

// text = 1*TEXT-CHAR
named_a!(pub text<'a, &'a str>, map_res!(take_while!(text_char),
    str::from_utf8
));

// TEXT-CHAR = <any CHAR except CR and LF>
pub fn text_char(c: u8) -> bool {
    c != b'\r' && c != b'\n'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_literal() {
        match string(b"{3}\r\nXYZ") {
            Ok((_, value)) => {
                assert_eq!(value, b"XYZ");
            }
            rsp => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_astring() {
        match astring(b"text ") {
            Ok((_, value)) => {
                assert_eq!(value, b"text");
            }
            rsp => panic!("unexpected response {:?}", rsp),
        }
    }
}
