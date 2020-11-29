/*
    Aim: wrap generated parser fns in struct
*/
use nom::{
    branch::alt,
    bytes::streaming::tag_no_case,
    bytes::streaming::{escaped_transform, is_a, is_not, tag, take, take_while_m_n},
    character::streaming::{alphanumeric0, alphanumeric1},
    combinator::map,
    combinator::opt,
    combinator::recognize,
    multi::{many0, many1},
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
    IResult,
};
use samotop_model::{
    common::Result,
    parser::Parser,
    smtp::{ReadControl, SmtpAddress, SmtpCommand, SmtpHost, SmtpPath},
};

static PARSER: SmtpParser = SmtpParser;

#[derive(Clone, Debug)]
pub struct SmtpParser;

impl Default for SmtpParser {
    fn default() -> SmtpParser {
        PARSER.clone()
    }
}

impl Parser for SmtpParser {
    fn command(&self, input: &[u8]) -> Result<SmtpCommand> {
        unimplemented!()
    }
    fn script(&self, input: &[u8]) -> Result<Vec<ReadControl>> {
        unimplemented!()
    }
    fn forward_path(&self, input: &[u8]) -> Result<SmtpPath> {
        unimplemented!()
    }
}

fn smtp_verb(i: &[u8]) -> IResult<&[u8], String> {
    use nom::AsChar;
    let (i, verb) = take_while_m_n(4, 20, u8::is_alpha)(i)?;
    Ok((i, String::from_utf8_lossy(verb).into()))
}
fn smtp_param(i: &[u8]) -> IResult<&[u8], SmtpParam> {
    alt((smtp_param_from_to, alt((smtp_esmtp_param, smtp_str_param))))(i)
}
fn smtp_str_param(i: &[u8]) -> IResult<&[u8], SmtpParam> {
    map(smtp_str, |v| SmtpParam::String(v))(i)
}
fn smtp_esmtp_param(i: &[u8]) -> IResult<&[u8], SmtpParam> {
    // keyword = (ALPHA / DIGIT) *(ALPHA / DIGIT / "-")
    let (i, keyword) = recognize(tuple((
        alphanumeric1,
        many0(alt((tag("-"), alphanumeric0))),
    )))(i)?;

    // 1*(%d33-60 / %d62-126)
    // any CHAR excluding "=", SP, and control
    // characters.  If this string is an email address,
    // i.e., a Mailbox, then the "xtext" syntax [32]
    // SHOULD be used.
    let (i, value) = opt(preceded(tag("="), is_a(VALUE)))(i)?;

    let keyword = String::from_utf8_lossy(keyword).to_string();
    let value = value.map(|v| String::from_utf8_lossy(v).into());

    let param = match value {
        Some(value) => SmtpParam::EsmtpValue(keyword, value),
        None => SmtpParam::Esmtp(keyword),
    };
    Ok((i, param))
}
fn smtp_param_from_to(i: &[u8]) -> IResult<&[u8], SmtpParam> {
    let (i, keyword) = terminated(alt((tag_no_case("from"), tag_no_case("to"))), tag(":"))(i)?;

    let (i, value) = preceded(many0(tag(" ")), is_a(VALUE))(i)?;

    let keyword = String::from_utf8_lossy(keyword).to_string();
    let value = String::from_utf8_lossy(value).into();

    let param = match keyword.as_bytes()[0] {
        b'f' | b'F' => SmtpParam::MailFrom(value),
        b't' | b'T' => SmtpParam::RcptTo(value),
        _ => unreachable!("only the two options are possible"),
    };
    Ok((i, param))
}
fn smtp_str(i: &[u8]) -> IResult<&[u8], String> {
    let (i, param) = alt((smtp_str_quoted, smtp_str_atom))(i)?;
    Ok((i, param))
}
fn smtp_str_atom(i: &[u8]) -> IResult<&[u8], String> {
    let (i, param) = is_a(ATEXT)(i)?;
    Ok((i, String::from_utf8_lossy(param.as_ref()).into()))
}
fn smtp_str_quoted(i: &[u8]) -> IResult<&[u8], String> {
    let (i, param) = delimited(
        tag("\""),
        escaped_transform(is_not("\r\n\"\\"), '\\', take(1usize)),
        tag("\""),
    )(i)?;
    let param = String::from_utf8(param).map_err(|_| {
        nom::Err::Failure(nom::error::Error::new(
            i,
            nom::error::ErrorKind::EscapedTransform,
        ))
    })?;
    Ok((i, param))
}

pub fn smtp_cmd(i: &[u8]) -> IResult<&[u8], (String, Vec<SmtpParam>)> {
    terminated(
        tuple((smtp_verb, many0(preceded(many1(tag(" ")), smtp_param)))),
        tag("\r\n"),
    )(i)
}

const ATEXT: &'static [u8] =
    b"!#$%&'*+-/01234567899=?ABCDEFGHIJKLMNOPQRSTUVWXYZ^_`abcdefghijklmnopqrstuvwxyz{|}~";
const VALUE: &'static [u8] =
    b"!\"#$%&'()*+,-./01234567899:;<>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~";

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SmtpParam {
    /// MAIL FROM:<xxx>
    MailFrom(String),
    /// RCPT TO:<yyy>
    RcptTo(String),
    /// ABCD
    Esmtp(String),
    /// ABCD=XYZ
    EsmtpValue(String, String),
    /// "xyz abc"
    String(String),
}

#[cfg(test)]
mod cmd_tests {
    use super::*;

    #[test]
    fn test_quit() {
        let res = smtp_cmd(b"QUIT\r\n".as_ref()).unwrap();
        assert_eq!(res.1, ("QUIT".to_owned(), vec![]));
        assert_eq!(res.0, b"");
    }

    #[test]
    fn test_param_quoted() {
        let res = smtp_cmd(b"HELP \"it's \\\"!@#$%^&*()- OK\"\r\n".as_ref()).unwrap();
        assert_eq!(
            res.1,
            (
                "HELP".to_owned(),
                vec![SmtpParam::String("it's \"!@#$%^&*()- OK".to_owned())]
            )
        );
        assert_eq!(res.0, b"");
    }

    #[test]
    fn test_mail() {
        let res = smtp_cmd(b"MAIL FROM:<a@b.x>\r\n".as_ref()).unwrap();
        assert_eq!(
            res.1,
            (
                "MAIL".to_owned(),
                vec![SmtpParam::MailFrom("<a@b.x>".to_owned())]
            )
        );
        assert_eq!(res.0, b"");
    }
}

#[cfg(test)]
mod str_tests {

    use super::*;

    #[test]
    fn test_str_atom() {
        let res = smtp_str_atom(b"abcd another".as_ref()).unwrap();
        assert_eq!(res.1, "abcd".to_owned());
        assert_eq!(res.0, b" another");
    }
    #[test]
    fn test_param_atom_complex() {
        let food = String::from_utf8_lossy(ATEXT).to_string() + " ";
        let res = smtp_str_atom(food.as_bytes()).unwrap();
        assert_eq!(res.1, String::from_utf8_lossy(ATEXT));
        assert_eq!(res.0, b" ");
    }

    #[test]
    fn test_str_quoted() {
        let res = smtp_str_quoted(b"\"abcd another\"".as_ref()).unwrap();
        assert_eq!(res.1, "abcd another".to_owned());
        assert_eq!(res.0, b"");
    }
    #[test]
    fn test_param_quoted_complex() {
        let res = smtp_str_quoted(b"\"it's \\\"!@#$%^&*()- OK\"".as_ref()).unwrap();
        assert_eq!(res.1, "it's \"!@#$%^&*()- OK".to_owned());
        assert_eq!(res.0, b"");
    }

    #[test]
    fn test_str() {
        let res = smtp_str(b"\"abcd another\"".as_ref()).unwrap();
        assert_eq!(res.1, "abcd another".to_owned());
        assert_eq!(res.0, b"");
    }

    #[test]
    fn string_parses_simple_ascii() {
        let result = smtp_str(b"abc\r\n").unwrap();
        assert_eq!(result.1, "abc".to_string());
    }

    #[test]
    fn string_parses_quotes_ascii() {
        let result = smtp_str(b"\"abc\"").unwrap();
        assert_eq!(result.1, "abc".to_string());
    }

    #[test]
    fn string_parses_quotes_ascii_with_quote() {
        let result = smtp_str(b"\"a\\\"bc\"").unwrap();
        assert_eq!(result.1, "a\"bc".to_string());
    }

    #[test]
    fn string_parses_quoted_utf8() {
        let result = smtp_str("\"ščřž\"".as_bytes()).unwrap();
        assert_eq!(result.1, "ščřž".to_string());
    }

    #[test]
    fn string_fails_on_invalid_utf8() {
        let result = smtp_str(b"\"\x80\x80\"");
        assert!(result.is_err());
    }
}
#[cfg(test)]
mod verb_tests {
    use super::*;

    fn u(inp: impl AsRef<[u8]>) -> String {
        String::from_utf8_lossy(inp.as_ref()).into()
    }

    #[test]
    fn test_quit() {
        let res = smtp_verb(b"QUIT\r\n".as_ref()).unwrap();
        assert_eq!(u(res.1), u(b"QUIT"));
        assert_eq!(res.0, b"\r\n");
    }
    #[test]
    fn test_starttls() {
        let res = smtp_verb(b"STARTTLS\r\n".as_ref()).unwrap();
        assert_eq!(u(res.1), u(b"STARTTLS"));
        assert_eq!(res.0, b"\r\n");
    }
    #[test]
    fn test_mail() {
        let res = smtp_verb(b"MAIL FROM:<a@b.x>\r\n".as_ref()).unwrap();
        assert_eq!(u(res.1), u(b"MAIL"));
        assert_eq!(res.0, b" FROM:<a@b.x>\r\n");
    }

    #[test]
    fn fail_non_asci() {
        let res = smtp_verb("ščřž der".as_bytes());
        assert!(res.is_err());
    }
}
