use samotop_model::{
    common::*,
    mail::MailSetup,
    parser::{ParseError, ParseResult, Parser},
    smtp::*,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct DataParserPeg {
    pub lmtp: bool,
}

impl MailSetup for DataParserPeg {
    fn setup(self, builder: &mut samotop_model::mail::Builder) {
        builder.data_parser.insert(0, Arc::new(self))
    }
}

impl Parser for DataParserPeg {
    fn parse_command<'i>(&self, input: &'i [u8]) -> ParseResult<'i, Box<dyn SmtpSessionCommand>> {
        let res = map_cmd(self.lmtp, grammar::data(input, true));
        trace!(
            "Parsed fresh {:?} from {:?}",
            res,
            String::from_utf8_lossy(input)
        );
        res
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct DataParserMidWayPeg {
    lmtp: bool,
}

impl Parser for DataParserMidWayPeg {
    fn parse_command<'i>(&self, input: &'i [u8]) -> ParseResult<'i, Box<dyn SmtpSessionCommand>> {
        let res = map_cmd(self.lmtp, grammar::data(input, false));
        trace!(
            "Parsed midway {:?} from {:?}",
            res,
            String::from_utf8_lossy(input)
        );
        res
    }
}

fn map_cmd(
    lmtp: bool,
    res: std::result::Result<ParseResult<Vec<u8>>, peg::error::ParseError<usize>>,
) -> ParseResult<Box<dyn SmtpSessionCommand>> {
    match res {
        Ok(Ok((i, data))) if data.is_empty() => Ok((i, Box::new(MailBodyEnd { lmtp }))),
        Ok(Ok((i, data))) if data.ends_with(b"\r\n") => {
            Ok((i, Box::new(MailBodyChunk(data, DataParserPeg { lmtp }))))
        }
        Ok(Ok((i, data))) => Ok((
            i,
            Box::new(MailBodyChunk(data, DataParserMidWayPeg { lmtp })),
        )),
        Ok(Err(e)) => Err(e),
        Err(e) => Err(ParseError::Failed(e.into())),
    }
}

fn utf8(bytes: &[u8]) -> std::result::Result<&str, &'static str> {
    std::str::from_utf8(bytes).map_err(|_e| "Invalid UTF-8")
}
fn utf8s(bytes: &[u8]) -> std::result::Result<String, &'static str> {
    utf8(bytes).map(|s| s.to_string())
}

peg::parser! {
    /// The parser takes advantage of keeping external state of reaching CR LF
    /// This state is passed as an argument. Caller detects CR LF end from output.
    /// The parser treats CR LF before final dot as part of the data
    ///    as otherwise the scheme is terribly ambiguous and complex.
    grammar grammar() for [u8] {

        pub rule data(crlf:bool) -> ParseResult<'input, Vec<u8>>
            = complete(crlf) / incomplete(crlf)

        rule complete(crlf:bool) -> ParseResult<'input, Vec<u8>>
            = s:( eof(crlf) / data_part(crlf) ) rest:$([_]*)
            {Ok((rest,s))}

        rule incomplete(crlf:bool) -> ParseResult<'input, Vec<u8>>
            = rest:$([_]*)
            {Err(ParseError::Incomplete)}

        rule eof(crlf:bool) ->  Vec<u8>
            =  b:$(".\r\n")
            { if crlf {vec![]} else {b.to_vec()} }

        rule data_part(crlf:bool) ->  Vec<u8>
            = s: ( escaped(crlf) / regular() )
            {s.into()}

        rule escaped(crlf:bool) -> String    = "." r:$(regular() / ".")
            {
                ?match (crlf, utf8s(r)) {
                    (_, Err(e)) => Err(e),
                    (true, Ok(r)) => Ok(r),
                    (false, Ok(r)) => Ok(format!(".{}",r)),
                }
            }
        rule regular() -> String = s:$( ( chr() / eols() )+ ) {?utf8s(s)}

        rule eols() = quiet!{ "\r"+ !("\r")&[_] / "\n" } / expected!("predictable new line chars CR LF")
        rule chr() = quiet!{![b'\r'|b'\n'|b'.'] [_]} / expected!("any char except CR LF and .")
    }
}

#[cfg(test)]
mod without_crlf {
    use super::*;
    use samotop_model::Result;
    const CRLF: bool = false;
    #[test]
    fn plain_chunk() -> Result<()> {
        match grammar::data(b"abcd", CRLF)? {
            Ok(([], b)) => assert_eq!(b, b"abcd".to_vec()),
            otherwise => panic!("Expected body chunk, got {:?}", otherwise),
        }
        Ok(())
    }

    #[test]
    fn crlf_chunk() -> Result<()> {
        match grammar::data(b"abcd\r\nxyz", CRLF)? {
            Ok(([], b)) => assert_eq!(b, b"abcd\r\nxyz".to_vec()),
            otherwise => panic!("Expected body chunk, got {:?}", otherwise),
        }
        Ok(())
    }

    #[test]
    fn lf_chunk() -> Result<()> {
        match grammar::data(b"abcd\nxyz", CRLF)? {
            Ok(([], b)) => assert_eq!(b, b"abcd\nxyz".to_vec()),
            otherwise => panic!("Expected body chunk, got {:?}", otherwise),
        }
        Ok(())
    }

    #[test]
    fn plain_eol() -> Result<()> {
        match grammar::data(b"foo\r\n", CRLF)? {
            Ok((b"", b)) if b == b"foo\r\n".to_vec() => {}
            otherwise => panic!("Expected foo, got {:?}", otherwise),
        }
        Ok(())
    }

    #[test]
    fn cr_chunk() -> Result<()> {
        match grammar::data(b"abcd\rxyz", CRLF)? {
            Ok(([], b)) => assert_eq!(b, b"abcd\rxyz".to_vec()),
            otherwise => panic!("Expected body chunk, got {:?}", otherwise),
        }
        Ok(())
    }

    #[test]
    fn mid_way_dot() -> Result<()> {
        match grammar::data(b".\r\n", CRLF)? {
            Ok((b"", b)) => assert_eq!(b, b".\r\n".to_vec()),
            otherwise => panic!("Expected dot, got {:?}", otherwise),
        }
        Ok(())
    }

    #[test]
    fn midway_dot_foo() -> Result<()> {
        match grammar::data(b".foo", CRLF)? {
            Ok(([], b)) if b == b".foo".to_vec() => {}
            otherwise => panic!("Expected dot foo, got {:?}", otherwise),
        }
        Ok(())
    }

    #[test]
    fn midway_dot_foo_crlf() -> Result<()> {
        match grammar::data(b".foo\r\n", CRLF)? {
            Ok(([], b)) if b == b".foo\r\n".to_vec() => {}
            otherwise => panic!("Expected dot foo crlf, got {:?}", otherwise),
        }
        Ok(())
    }

    #[test]
    fn mid_way_lflf() -> Result<()> {
        match grammar::data(b"\n\nfoo", CRLF)? {
            Ok(([], b)) => assert_eq!(b, b"\n\nfoo".to_vec()),
            otherwise => panic!("Expected chunk, got {:?}", otherwise),
        }
        Ok(())
    }
    #[test]
    fn complex() {
        let input = b"\r\n..\r\nxoxo\r\n.\r\n";
        let (input, b) = grammar::data(input, CRLF).unwrap().unwrap();
        assert_eq!(b, b"\r\n".to_vec());
        let (input, b) = grammar::data(input, b.ends_with(b"\r\n")).unwrap().unwrap();
        assert_eq!(b, b".".to_vec());
        let (input, b) = grammar::data(input, b.ends_with(b"\r\n")).unwrap().unwrap();
        assert_eq!(b, b"\r\nxoxo\r\n".to_vec());
        let (input, b) = grammar::data(input, b.ends_with(b"\r\n")).unwrap().unwrap();
        assert_eq!(b, b"".to_vec());
        assert!(input.is_empty());
    }
    #[test]
    fn full_dot_stop() -> Result<()> {
        match grammar::data(b"\r\n.\r\n", CRLF)? {
            Ok((b".\r\n", b)) => assert_eq!(b, b"\r\n".to_vec()),
            otherwise => panic!("Expected crlf, got {:?}", otherwise),
        }
        Ok(())
    }
    #[test]
    fn mid_way_dot_stop() -> Result<()> {
        match grammar::data(b".\r\n", CRLF)? {
            Ok((b"", b)) => assert_eq!(b, b".\r\n".to_vec()),
            otherwise => panic!("Expected chunk, got {:?}", otherwise),
        }
        Ok(())
    }
    #[test]
    fn get_crlf() -> Result<()> {
        match grammar::data(b"\r\n", CRLF)? {
            Ok(([], b)) => assert_eq!(b, b"\r\n".to_vec()),
            otherwise => panic!("Expected crlf, got {:?}", otherwise),
        }
        Ok(())
    }
    #[test]
    fn get_crlf_dot() -> Result<()> {
        match grammar::data(b"\r\n.", CRLF)? {
            Ok((b".", b)) => assert_eq!(b, b"\r\n".to_vec()),
            otherwise => panic!("Expected crlf, got {:?}", otherwise),
        }
        Ok(())
    }
    #[test]
    fn incomplete_cr() -> Result<()> {
        match grammar::data(b"\r", CRLF)? {
            Err(ParseError::Incomplete) => {}
            otherwise => panic!("Expected incomplete, got {:?}", otherwise),
        }
        Ok(())
    }
    #[test]
    fn incomplete_empty() -> Result<()> {
        match grammar::data(b"", CRLF)? {
            Err(ParseError::Incomplete) => {}
            otherwise => panic!("Expected incomplete, got {:?}", otherwise),
        }
        Ok(())
    }
}

#[cfg(test)]
mod after_crlf {
    use super::*;
    use samotop_model::Result;
    const CRLF: bool = true;
    #[test]
    fn complex() {
        let input = b"\r\n..\r\nxoxo\r\n.\r\n";
        let (input, b) = grammar::data(input, CRLF).unwrap().unwrap();
        assert_eq!(b, b"\r\n".to_vec());
        let (input, b) = grammar::data(input, b.ends_with(b"\r\n")).unwrap().unwrap();
        assert_eq!(b, b".".to_vec());
        let (input, b) = grammar::data(input, b.ends_with(b"\r\n")).unwrap().unwrap();
        assert_eq!(b, b"\r\nxoxo\r\n".to_vec());
        assert_eq!(input, b".\r\n".to_vec());
        let (input, b) = grammar::data(input, true).unwrap().unwrap();
        assert_eq!(b, b"".to_vec(), "input: {:?}", input);
        assert!(input.is_empty());
    }

    #[test]
    fn plain_chunk() -> Result<()> {
        match grammar::data(b"abcd", CRLF)? {
            Ok(([], b)) => assert_eq!(b, b"abcd".to_vec()),
            otherwise => panic!("Expected body chunk, got {:?}", otherwise),
        }
        Ok(())
    }

    #[test]
    fn ignores_command() -> Result<()> {
        match grammar::data(b".\r\nquit\r\n\r\n", CRLF)? {
            Ok((b"quit\r\n\r\n", b)) => assert_eq!(b, b"".to_vec()),
            otherwise => panic!("Expected end, got {:?}", otherwise),
        }
        Ok(())
    }

    #[test]
    fn crlf_chunk() -> Result<()> {
        match grammar::data(b"abcd\r\nxyz", CRLF)? {
            Ok(([], b)) => assert_eq!(b, b"abcd\r\nxyz".to_vec()),
            otherwise => panic!("Expected body chunk, got {:?}", otherwise),
        }
        Ok(())
    }

    #[test]
    fn lf_chunk() -> Result<()> {
        match grammar::data(b"abcd\nxyz", CRLF)? {
            Ok(([], b)) => assert_eq!(b, b"abcd\nxyz".to_vec()),
            otherwise => panic!("Expected body chunk, got {:?}", otherwise),
        }
        Ok(())
    }

    #[test]
    fn plain_eol() -> Result<()> {
        match grammar::data(b"foo\r\n", CRLF)? {
            Ok((b"", b)) if b == b"foo\r\n".to_vec() => {}
            otherwise => panic!("Expected foo crlf, got {:?}", otherwise),
        }
        Ok(())
    }

    #[test]
    fn cr_chunk() -> Result<()> {
        match grammar::data(b"abcd\rxyz", CRLF)? {
            Ok(([], b)) => assert_eq!(b, b"abcd\rxyz".to_vec()),
            otherwise => panic!("Expected body chunk, got {:?}", otherwise),
        }
        Ok(())
    }

    #[test]
    fn dot_stop() -> Result<()> {
        match grammar::data(b".\r\n", CRLF)? {
            Ok(([], b)) => {
                assert!(b.is_empty());
                assert_eq!(b, b"");
            }
            otherwise => panic!("Expected end, got {:?}", otherwise),
        }
        Ok(())
    }
    #[test]
    fn dot_stop_full() -> Result<()> {
        match grammar::data(b"\r\n.\r\n", CRLF)? {
            Ok((b".\r\n", b)) => assert_eq!(b, b"\r\n".to_vec()),
            otherwise => panic!("Expected crlf, got {:?}", otherwise),
        }
        Ok(())
    }

    #[test]
    fn dot_escape() -> Result<()> {
        match grammar::data(b".foo", CRLF)? {
            Ok(([], b)) if b == b"foo".to_vec() => {}
            otherwise => panic!("Expected foo, got {:?}", otherwise),
        }
        Ok(())
    }

    #[test]
    fn dot_escape_crlf() -> Result<()> {
        match grammar::data(b".foo\r\n", CRLF)? {
            Ok((b"", b)) if b == b"foo\r\n".to_vec() => {}
            otherwise => panic!("Expected foo crlf, got {:?}", otherwise),
        }
        Ok(())
    }

    #[test]
    fn trailing_lf() -> Result<()> {
        match grammar::data(b"\n\r\n.\r\n", CRLF)? {
            Ok((b".\r\n", b)) if b == b"\n\r\n".to_vec() => {}
            otherwise => panic!("Expected lf, got {:?}", otherwise),
        }
        Ok(())
    }
    #[test]
    fn trailing_cr() -> Result<()> {
        match grammar::data(b"\r\r\n.\r\n", CRLF)? {
            Ok((b".\r\n", b)) if b == b"\r\r\n".to_vec() => {}
            otherwise => panic!("Expected cr, got {:?}", otherwise),
        }
        Ok(())
    }
    #[test]
    fn get_crlf() -> Result<()> {
        match grammar::data(b"\r\n", CRLF)? {
            Ok((b"", b)) if b == b"\r\n".to_vec() => {}
            otherwise => panic!("Expected crlf, got {:?}", otherwise),
        }
        Ok(())
    }
    #[test]
    fn get_crlf_dot() -> Result<()> {
        match grammar::data(b"\r\n.", CRLF)? {
            Ok((b".", b)) if b == b"\r\n".to_vec() => {}
            otherwise => panic!("Expected crlf, got {:?}", otherwise),
        }
        Ok(())
    }
    #[test]
    fn incomplete_cr() -> Result<()> {
        match grammar::data(b"\r", CRLF)? {
            Err(ParseError::Incomplete) => {}
            otherwise => panic!("Expected incomplete, got {:?}", otherwise),
        }
        Ok(())
    }
    #[test]
    fn incomplete_dot() -> Result<()> {
        match grammar::data(b".", CRLF)? {
            Err(ParseError::Incomplete) => {}
            otherwise => panic!("Expected incomplete, got {:?}", otherwise),
        }
        Ok(())
    }
    #[test]
    fn incomplete_dot_cr() -> Result<()> {
        match grammar::data(b".\r", CRLF)? {
            Err(ParseError::Incomplete) => {}
            otherwise => panic!("Expected incomplete, got {:?}", otherwise),
        }
        Ok(())
    }
    #[test]
    fn incomplete_empty() -> Result<()> {
        match grammar::data(b"", CRLF)? {
            Err(ParseError::Incomplete) => {}
            otherwise => panic!("Expected incomplete, got {:?}", otherwise),
        }
        Ok(())
    }
}
