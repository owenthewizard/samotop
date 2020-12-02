use crate::data::DataParserPeg;
use samotop_model::parser::{ParseError, ParseResult};
use samotop_model::smtp::*;
use samotop_model::{
    common::Arc, mail::MailSetup, parser::Parser, smtp::SmtpPath, smtp::SmtpSessionCommand, Error,
};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

pub mod grammar {
    pub(crate) use super::smtp_grammar::*;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SmtpParserPeg;

impl Parser for SmtpParserPeg {
    fn parse_command<'i>(&self, input: &'i [u8]) -> ParseResult<'i, Box<dyn SmtpSessionCommand>> {
        if input.is_empty() {
            return Err(ParseError::Incomplete);
        }
        match grammar::command(input) {
            Err(e) => Err(ParseError::Failed(e.into())),
            Ok(Err(e)) => Err(e),
            Ok(Ok((i, cmd))) => Ok((
                i,
                match cmd {
                    SmtpCommand::Helo(helo) => match helo {
                        trad @ SmtpHelo::Ehlo(_) | trad @ SmtpHelo::Helo(_) => Box::new(trad),
                        _ => Box::new(SmtpUnknownCommand::default()),
                    },
                    _ => Box::new(cmd),
                },
            )),
        }
    }
}

impl MailSetup for SmtpParserPeg {
    fn setup(self, builder: &mut samotop_model::mail::Builder) {
        builder.command_parser.insert(0, Arc::new(self));
        builder
            .data_parser
            .insert(0, Arc::new(DataParserPeg { lmtp: false }));
    }
}

impl SmtpParserPeg {
    pub fn forward_path<'i>(&self, input: &'i [u8]) -> ParseResult<'i, SmtpPath> {
        Self::map(grammar::path_forward(input), b"")
    }
    fn map<T, E>(myres: std::result::Result<T, E>, input: &[u8]) -> ParseResult<T>
    where
        E: Into<Error>,
    {
        match myres {
            Ok(item) => Ok((input, item)),
            Err(e) => Err(ParseError::Mismatch(e.into())),
        }
    }
}

fn utf8(bytes: &[u8]) -> std::result::Result<&str, &'static str> {
    std::str::from_utf8(bytes).map_err(|_e| "Invalid UTF-8")
}
fn utf8s(bytes: &[u8]) -> std::result::Result<String, &'static str> {
    utf8(bytes).map(|s| s.to_string())
}

peg::parser! {
    grammar smtp_grammar() for [u8] {

        // https://github.com/kevinmehall/rust-peg/issues/216
        rule i(literal: &'static str)
            = input:$([_]*<{literal.len()}>)
            {? if input.eq_ignore_ascii_case(literal.as_bytes()) { Ok(()) } else { Err(literal) } }

        pub rule command() -> ParseResult<'input, SmtpCommand>
            = cmd:(valid_command() / invalid_command() / incomplete_command())
            {cmd}

        pub rule valid_command() -> ParseResult<'input, SmtpCommand>
            = cmd: (cmd_starttls() /
                cmd_helo() /
                cmd_ehlo() /
                cmd_lhlo() /
                cmd_mail() /
                cmd_send() /
                cmd_soml() /
                cmd_saml() /
                cmd_rcpt() /
                cmd_data() /
                cmd_rset() /
                cmd_quit() /
                cmd_noop() /
                cmd_turn() /
                cmd_vrfy() /
                cmd_expn() /
                cmd_help()) rest:$([_]*)
            {Ok((rest, cmd))}

        rule incomplete_command() -> ParseResult<'input, SmtpCommand>
            = s:$(quiet!{ [_]+ } / expected!("incomplete input"))
            {Err(ParseError::Incomplete)}

        rule invalid_command() -> ParseResult<'input, SmtpCommand>
            = s:$(quiet!{ "\n" / (![b'\n'][_]) + "\n" } / expected!("invalid input"))
            {ParseResult::Err(ParseError::Mismatch("unrecognized command".into()))}

        pub rule cmd_starttls() -> SmtpCommand
            = i("starttls") NL()
            { SmtpCommand::StartTls }

        pub rule cmd_quit() -> SmtpCommand
            = i("quit") NL()
            { SmtpCommand::Quit }

        pub rule cmd_rset() -> SmtpCommand
            = i("rset") NL()
            { SmtpCommand::Rset }

        pub rule cmd_data() -> SmtpCommand
            = i("data") NL()
            { SmtpCommand::Data }

        pub rule cmd_turn() -> SmtpCommand
            = i("turn") NL()
            { SmtpCommand::Turn }

        pub rule cmd_mail() -> SmtpCommand
            = i("mail from:") p:path_reverse() s:strparam()* NL()
            { SmtpCommand::Mail(SmtpMail::Mail(p, s)) }
        pub rule cmd_send() ->SmtpCommand
            = i("send from:") p:path_reverse() s:strparam()* NL()
            { SmtpCommand::Mail(SmtpMail::Send(p, s)) }
        pub rule cmd_soml() -> SmtpCommand
            = i("soml from:") p:path_reverse() s:strparam()* NL()
            { SmtpCommand::Mail(SmtpMail::Soml(p, s)) }
        pub rule cmd_saml() -> SmtpCommand
            = i("saml from:") p:path_reverse() s:strparam()* NL()
            { SmtpCommand::Mail(SmtpMail::Saml(p, s)) }

        pub rule cmd_rcpt() -> SmtpCommand
            = i("rcpt to:") p:path_forward() s:strparam()* NL()
            { SmtpCommand::Rcpt(SmtpRcpt(p, s)) }

        pub rule cmd_helo() -> SmtpCommand
            = i("helo") _ h:host() NL()
            { SmtpCommand::Helo(SmtpHelo::Helo(h)) }

        pub rule cmd_ehlo() -> SmtpCommand
            = i("ehlo") _ h:host() NL()
            { SmtpCommand::Helo(SmtpHelo::Ehlo(h)) }

        pub rule cmd_lhlo() -> SmtpCommand
            = i("lhlo") _ h:host() NL()
            { SmtpCommand::Helo(SmtpHelo::Lhlo(h)) }

        pub rule cmd_vrfy() -> SmtpCommand
            = i("vrfy") s:strparam() NL()
            { SmtpCommand::Vrfy(s) }

        pub rule cmd_expn() -> SmtpCommand
            = i("expn") s:strparam() NL()
            { SmtpCommand::Expn(s) }

        pub rule cmd_noop() -> SmtpCommand
            = i("noop") s:strparam()* NL()
            { SmtpCommand::Noop(s) }

        pub rule cmd_help() -> SmtpCommand
            = i("help") s:strparam()* NL()
            { SmtpCommand::Help(s) }

        pub rule path_forward() -> SmtpPath
            = path_relay() / path_direct() / path_postmaster()
        pub rule path_reverse() -> SmtpPath
            = path_relay() / path_direct() / path_null()

        rule path_relay() -> SmtpPath
            = "<" h:athost()+ a:address() ">"
            { SmtpPath::Relay(h, a) }

        rule path_direct() -> SmtpPath
            = "<" a:address() ">"
            { SmtpPath::Direct(a) }

        rule path_postmaster() -> SmtpPath
            = i("<postmaster>")
            { SmtpPath::Postmaster }

        rule path_null() -> SmtpPath
            = "<>"
            { SmtpPath::Null }

        pub rule address() -> SmtpAddress
            = s:dot_string() "@" h:host()
            { SmtpAddress::Mailbox (s, h) }

        rule athost() -> SmtpHost
            = "@" h:host() (&",@" "," / ":")
            { h }

        rule strparam() -> String
            = _ s:string()
            { s }

        pub rule host() -> SmtpHost
            = host_numeric() /
            host_ipv4() /
            host_ipv6() /
            host_other() /
            host_domain()

        rule host_domain() -> SmtpHost
            = s:$( label() ("." label())* )
            {? utf8s(s).map(SmtpHost::Domain) }
        rule domain() = quiet!{label() ("." label())*} / expected!("domain name")
        rule label() = [b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9'] [b'-' | b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9']*

        rule host_numeric() -> SmtpHost
            = "#" s:$([b'0'..=b'9']+ / expected!("ipv4 number"))
            { match u32::from_str(utf8(s).expect("ASCII")) {
                Ok(ip) => SmtpHost::Ipv4(Ipv4Addr::from(ip)),
                Err(e) => SmtpHost::Invalid{label:"numeric".to_string(), literal:utf8s(s).expect("ASCII")},
            } }

        rule host_ipv4() -> SmtpHost
            = "[" s:$(ipv4addr()) "]"
            { match Ipv4Addr::from_str(utf8(s).expect("ASCII")) {
                Ok(ip) => SmtpHost::Ipv4(ip),
                Err(e) => SmtpHost::Invalid{label:"ipv4".to_string(), literal:utf8s(s).expect("ASCII")},
            } }
        rule ipv4addr() = quiet!{ipv4part() "." ipv4part() "." ipv4part() "." ipv4part()} / expected!("ipv4 address")
        rule ipv4part() = "25" [b'0'..=b'5'] /
            "2" [b'0'..=b'4'] [b'0'..=b'9'] /
            [b'0'..=b'1'] [b'0'..=b'9'] [b'0'..=b'9']? /
            [b'0'..=b'9'] [b'0'..=b'9']?

        rule host_ipv6() -> SmtpHost
            = l:$(i("IPv6")) ":" s:$(ipv6addr())
            { match Ipv6Addr::from_str(utf8(s).expect("ASCII")) {
                Ok(ip) => SmtpHost::Ipv6(ip),
                Err(e) => SmtpHost::Invalid{label:utf8s(l).expect("ASCII"), literal:utf8s(s).expect("ASCII")},
            } }
        rule ipv6addr() = quiet!{[b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F' | b':' | b'.']+} / expected!("ipv6 address")

        rule host_other() -> SmtpHost
            = l:string() ":" s:string()
            { SmtpHost::Other{label:l, literal:s} }

        pub rule string() -> String
            = str_quoted() / str_plain()

        pub rule dot_string() -> String
            = str_quoted() / str_dot_plain()

        rule str_plain() -> String
            = s:(chr()*)
            {? utf8s(&s[..]) }

        rule str_dot_plain() -> String
            = s:(chr_dot()*)
            {? utf8s(&s[..]) }

        rule str_quoted() -> String
            = [b'"'] s:(qchar()*) [b'"']
            {? utf8s(&s[..]) }

        rule qchar() -> u8
            = qchar_regular() / char_special()

        rule qchar_regular() -> u8
            = b:$(quiet!{!("\"" / "\\" / "\r" / "\n") [_]} / expected!("quoted character"))
            {debug_assert!(b.len()==1); b[0]}

        rule chr() -> u8
            = char_regular() / char_special()
        rule chr_dot() -> u8
            = char_regular() / char_special() / dot()

        rule char_regular() -> u8
            = b:$(quiet!{[b'-' | b'!' | b'#' | b'$' | b'%' | b'&' |
                b'\'' | b'*' | b'+' | b'-' | b'`' | b'/' |
                    b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' |
                    b'=' | b'?' | b'~' | b'^' | b'_' | b'{' | b'}' | b'|' | 0x80..=0xFF
            ]} / expected!("regular character"))
            {debug_assert!(b.len()==1); b[0]}

        rule char_special() -> u8
            = ignore:("\\") b:$(quiet!{[_]} / expected!("special character"))
            {debug_assert!(b.len()==1); b[0]}

        rule dot() -> u8
            = b:$(".")
            {debug_assert!(b.len()==1); b[0]}

        rule NL() = quiet!{"\r\n" / "\n"} / expected!("{NL}")
        rule _() = quiet!{" "} / expected!("{SP}")
        rule __() = quiet!{_ / "\t"} / expected!("{WS}")
    }
}

#[cfg(test)]
mod tests {
    use super::grammar::*;
    use super::*;
    use samotop_model::Result;

    #[test]
    fn command_parses_unknown_command() {
        let result = command(b"sOmE other command\r\n");
        match result {
            Ok(Err(ParseError::Mismatch(_))) => { /*OK*/ }
            otherwise => panic!("Expected mismatch, got {:?}", otherwise),
        }
    }

    #[test]
    fn cmd_parses_valid_mail_from() {
        let result = command(b"mail from:<here.there@everywhere.net>\r\n")
            .unwrap()
            .unwrap();
        assert_eq!(
            result.1,
            SmtpCommand::Mail(SmtpMail::Mail(
                SmtpPath::Direct(SmtpAddress::Mailbox(
                    "here.there".to_owned(),
                    SmtpHost::Domain("everywhere.net".to_owned())
                )),
                vec![]
            ))
        );
    }

    #[test]
    fn host_parses_unknown_host() {
        let result = host(b"who:what").unwrap();
        assert_eq!(
            result,
            SmtpHost::Other {
                label: "who".to_string(),
                literal: "what".to_string(),
            }
        );
    }

    #[test]
    fn cmd_parser_starttls() {
        let result = command(b"STARTTLS\r\n").unwrap().unwrap();
        assert_eq!(result.1, SmtpCommand::StartTls);
    }

    #[test]
    fn script_parses_whitespace_line() {
        let result = command(b"   \r\n\t\t\r\n");
        assert!(result.is_err());
    }

    #[test]
    fn session_parses_helo() {
        let input = b"helo domain.com\r\n";
        let cmd = command(input).unwrap().unwrap().1;
        assert_eq!(cmd.verb(), "HELO");
    }

    #[test]
    fn session_parses_data() -> Result<()> {
        let input = "DATA\r\n ěšě\r\nš\nčš".as_bytes();
        let cmd = command(input)??.1;
        assert_eq!(cmd.verb(), "DATA");
        Ok(())
    }

    #[test]
    fn session_parses_wrong_newline() {
        let cmd = command(b"QUIT\nQUIT\r\nquit\r\n").unwrap().unwrap();
        assert_eq!(cmd, (b"QUIT\r\nquit\r\n".as_ref(), SmtpCommand::Quit));
    }

    #[test]
    fn session_parses_incomplete_command() {
        let cmd = command(b"QUIT\r\nQUI").unwrap().unwrap();
        assert_eq!(cmd, (b"QUI".as_ref(), SmtpCommand::Quit));
    }

    #[test]
    fn session_parses_valid_utf8() {
        let cmd = command("Help \"ěščř\"\r\n".as_bytes()).unwrap().unwrap();
        assert_eq!(
            cmd,
            (b"".as_ref(), SmtpCommand::Help(vec!["ěščř".to_owned()]))
        );
    }

    #[test]
    fn session_parses_invalid_utf8() {
        let result = command(b"Help \"\x80\x80\"\r\n");
        match result {
            Ok(Err(ParseError::Mismatch(_))) => { /*OK*/ }
            otherwise => panic!("Expected mismatch, got {:?}", otherwise),
        }
    }

    #[test]
    fn session_parses_helo_mail_rcpt_quit() {
        let cmd = command(
            concat!(
                "helo domain.com\r\n",
                "mail from:<me@there.net>\r\n",
                "rcpt to:<@relay.net:him@unreachable.local>\r\n",
                "quit\r\n"
            )
            .as_bytes(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            cmd,
            (
                concat!(
                    "mail from:<me@there.net>\r\n",
                    "rcpt to:<@relay.net:him@unreachable.local>\r\n",
                    "quit\r\n"
                )
                .as_bytes(),
                SmtpCommand::Helo(SmtpHelo::Helo(SmtpHost::Domain("domain.com".to_owned())))
            )
        );
    }

    #[test]
    fn string_parses_simple_ascii() {
        let result = string(b"abc").unwrap();
        assert_eq!(result, "abc".to_string());
    }

    #[test]
    fn string_parses_quotes_ascii() {
        let result = string(b"\"abc\"").unwrap();
        assert_eq!(result, "abc".to_string());
    }

    #[test]
    fn string_parses_quotes_ascii_with_quote() {
        let result = string(b"\"a\\\"bc\"").unwrap();
        assert_eq!(result, "a\"bc".to_string());
    }

    #[test]
    fn string_parses_quoted_utf8() {
        let result = string("\"ščřž\"".as_bytes()).unwrap();
        assert_eq!(result, "ščřž".to_string());
    }

    #[test]
    fn string_parses_simple_utf8() {
        let result = string("ščřž".as_bytes()).unwrap();
        assert_eq!(result, "ščřž".to_string());
    }

    #[test]
    fn string_fails_on_invalid_utf8() {
        let result = string(b"\"\x80\x80\"");
        assert!(result.is_err());
    }
}
