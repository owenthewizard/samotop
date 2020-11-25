/*
    Aim: wrap generated parser fns in struct
*/
use nom::{
    branch::alt,
    bytes::streaming::{tag, take_while_m_n},
    combinator::map,
    multi::many0,
    sequence::{terminated, tuple},
    IResult,
};
use samotop_model::{
    common::Result,
    parser::Parser,
    smtp::{ReadControl, SmtpCommand, SmtpPath},
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

fn smtp_verb(i: &[u8]) -> IResult<&[u8], &[u8]> {
    use nom::AsChar;
    let (i, verb) = take_while_m_n(4, 20, u8::is_alpha)(i)?;
    Ok((i, verb))
}

fn smtp_param(i: &[u8]) -> IResult<&[u8], &[u8]> {
    let (i, _) = nom::bytes::streaming::is_a(" ")(i)?;
    let (i, param) = nom::bytes::streaming::is_not(" \r\n")(i)?;
    Ok((i, param))
}

fn smtp_cmd(i: &[u8]) -> IResult<&[u8], (&str, Vec<&str>)> {
    map(
        terminated(tuple((smtp_verb, many0(smtp_param))), tag("\r\n")),
        |t| {
            (
                std::str::from_utf8(t.0).unwrap(),
                t.1.iter()
                    .map(|p| std::str::from_utf8(p).unwrap())
                    .collect(),
            )
        },
    )(i)
}

#[cfg(test)]
mod cmd_tests {
    use super::*;

    #[test]
    fn test_quit() {
        let res = smtp_cmd(b"QUIT\r\n".as_ref()).unwrap();
        assert_eq!(res.1, ("QUIT", vec![]));
        assert_eq!(res.0, b"");
    }

    #[test]
    fn test_mail() {
        let res = smtp_cmd(b"MAIL FROM:<a@b.x>\r\n".as_ref()).unwrap();
        assert_eq!(res.1, ("MAIL", vec!["FROM:<a@b.x>"]));
        assert_eq!(res.0, b"");
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
}
