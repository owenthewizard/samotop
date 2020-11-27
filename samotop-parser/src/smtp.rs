use samotop_model::smtp::*;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

fn utf8(bytes: &[u8]) -> std::result::Result<&str, &'static str> {
    std::str::from_utf8(bytes).map_err(|_e| "Invalid UTF-8")
}
fn utf8s(bytes: &[u8]) -> std::result::Result<String, &'static str> {
    utf8(bytes).map(|s| s.to_string())
}

peg::parser! {
    pub grammar grammar() for [u8] {
        pub rule session() -> Vec<ReadControl>
            = __* i:script()
            { i }

        pub rule script() -> Vec<ReadControl>
            = input()+

        pub rule input() -> ReadControl
            = inp_none() / inp_command() / inp_invalid() /// inp_incomplete()
        pub rule inp_command() -> ReadControl
            = start:position!() c:command() end:position!()
            { ReadControl::Command( Box::new(c), Vec::from(&__input[start..end])) }
        pub rule inp_none() -> ReadControl
            =  s:$(NL() / __* NL())
            { ReadControl::Empty(Vec::from(s)) }
        pub rule inp_invalid() -> ReadControl
            =  s:$( quiet!{ str_invalid() / str_incomplete() } / expected!("invalid input") )
            { ReadControl::Raw( Vec::from(s)) }

        rule str_invalid() = quiet!{ "\n" / (![b'\n'][_]) + "\n" } / expected!("invalid input")
        rule str_incomplete() = quiet!{ [_]+ } / expected!("incomplete input")

        // https://github.com/kevinmehall/rust-peg/issues/216
        rule i(literal: &'static str)
            = input:$([_]*<{literal.len()}>)
            {? if input.eq_ignore_ascii_case(literal.as_bytes()) { Ok(()) } else { Err(literal) } }

        pub rule command() -> SmtpCommand
            = cmd_starttls() /
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
            cmd_help()

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
        pub rule cmd_send() -> SmtpCommand
            = i("send from:") p:path_reverse() s:strparam()* NL()
            { SmtpCommand::Mail(SmtpMail::Send(p, s)) }
        pub rule cmd_soml() -> SmtpCommand
            = i("soml from:") p:path_reverse() s:strparam()* NL()
            { SmtpCommand::Mail(SmtpMail::Soml(p, s)) }
        pub rule cmd_saml() -> SmtpCommand
            = i("saml from:") p:path_reverse() s:strparam()* NL()
            { SmtpCommand::Mail(SmtpMail::Saml(p, s)) }

        pub rule cmd_rcpt() -> SmtpCommand
            = i("rcpt to:") p:path_forward() NL()
            { SmtpCommand::Rcpt(p) }

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
    use super::ReadControl::*;
    use super::*;
    use crate::grammar::*;
    use samotop_model::Result;

    fn b(bytes: impl AsRef<[u8]>) -> Vec<u8> {
        Vec::from(bytes.as_ref())
    }

    #[test]
    fn script_parses_unknown_command() {
        let result = script(b"sOmE other command\r\n").unwrap();

        match result.as_slice() {
            [Raw(bytes)] => assert_eq!(bytes.as_slice(), b"sOmE other command\r\n"),
            res => panic!("invalid result. Expected Raw, got {:?}", res),
        }
    }

    #[test]
    fn cmd_parses_valid_mail_from() {
        let result = command(b"mail from:<here.there@everywhere.net>\r\n").unwrap();
        assert_eq!(
            result,
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
        let result = command(b"STARTTLS\r\n").unwrap();
        assert_eq!(result, SmtpCommand::StartTls);
    }

    #[test]
    fn script_parses_whitespace_line() {
        let result = script(b"   \r\n\t\t\r\n").unwrap();

        match result.as_slice() {
            [Empty(empty1), Empty(empty2)] => {
                assert_eq!(empty1.as_slice(), b"   \r\n");
                assert_eq!(empty2.as_slice(), b"\t\t\r\n");
            }
            res => panic!("invalid result. Expected 2x empty, got {:?}", res),
        }
    }

    #[test]
    fn session_parses_helo() {
        let input = b"helo domain.com\r\n";
        let result = session(input).unwrap();

        match result.as_slice() {
            [Command(cmd, _)] => assert_eq!(cmd.verb(), "HELO"),
            res => panic!("invalid result. Expected helo, got {:?}", res),
        }
    }

    #[test]
    fn session_parses_data() -> Result<()> {
        let input = "DATA\r\n ěšě\r\nš\nčš".as_bytes();
        let result = session(input)?;
        match result.as_slice() {
            [Command(cmd, _), Raw(b1), Raw(b2), Raw(b3)] => {
                assert_eq!(cmd.verb(), "DATA");
                assert_eq!(b1.as_slice(), " ěšě\r\n".as_bytes());
                assert_eq!(b2.as_slice(), "š\n".as_bytes());
                assert_eq!(b3.as_slice(), "čš".as_bytes());
            }
            res => panic!("invalid result. Expected command and raw, got {:?}", res),
        }
        Ok(())
    }

    #[test]
    fn session_parses_wrong_newline() {
        let result = session(b"QUIT\nQUIT\r\nquit\r\n").unwrap();
        match result.as_slice() {
            [Command(cmd1, _), Command(cmd2, _), Command(cmd3, _)] => {
                assert_eq!(cmd1.verb(), "QUIT");
                assert_eq!(cmd2.verb(), "QUIT");
                assert_eq!(cmd3.verb(), "QUIT");
            }
            res => panic!("invalid result. Expected commands, got {:?}", res),
        }
    }

    #[test]
    fn session_parses_incomplete_command() {
        let result = session(b"QUIT\r\nQUI").unwrap();
        match result.as_slice() {
            [Command(cmd1, _), Raw(bytes2)] => {
                assert_eq!(cmd1.verb(), "QUIT");
                assert_eq!(bytes2.as_slice(), b"QUI");
            }
            res => panic!(
                "invalid result. Expected command and incomplete, got {:?}",
                res
            ),
        }
    }

    #[test]
    fn session_parses_valid_utf8() {
        let result = session("Help \"ěščř\"\r\n".as_bytes()).unwrap();

        match result.as_slice() {
            [Command(cmd1, bytes1)] => {
                assert_eq!(cmd1.verb(), "HELP");
                assert_eq!(bytes1.as_slice(), "Help \"ěščř\"\r\n".as_bytes());
            }
            res => panic!(
                "invalid result. Expected command and incomplete, got {:?}",
                res
            ),
        }
    }

    #[test]
    fn session_parses_invalid_utf8() {
        let result = session(b"Help \"\x80\x80\"\r\n").unwrap();

        match result.as_slice() {
            [Raw(bytes1)] => {
                assert_eq!(bytes1.as_slice(), b"Help \"\x80\x80\"\r\n");
            }
            res => panic!("invalid result. Expected raw, got {:?}", res),
        }
    }

    #[test]
    fn session_parses_helo_mail_rcpt_quit() {
        let result = session(
            concat!(
                "helo domain.com\r\n",
                "mail from:<me@there.net>\r\n",
                "rcpt to:<@relay.net:him@unreachable.local>\r\n",
                "quit\r\n"
            )
            .as_bytes(),
        )
        .unwrap();
        match result.as_slice() {
            [Command(cmd1, _), Command(cmd2, _), Command(cmd3, _), Command(cmd4, _)] => {
                assert_eq!(cmd1.verb(), "HELO");
                assert_eq!(cmd2.verb(), "MAIL");
                assert_eq!(cmd3.verb(), "RCPT");
                assert_eq!(cmd4.verb(), "QUIT");
            }
            res => panic!("invalid result. Expected commands, got {:?}", res),
        }
        // use super::SmtpCommand::*;
        // use super::SmtpHost::*;
        // assert_eq!(
        //     result,
        //     vec![
        //         Command(
        //             Helo(SmtpHelo::Helo(Domain("domain.com".to_string()))),
        //             b("helo domain.com\r\n")
        //         ),
        //         Command(
        //             Mail(SmtpMail::Mail(
        //                 SmtpPath::Direct(SmtpAddress::Mailbox(
        //                     "me".to_string(),
        //                     Domain("there.net".to_string()),
        //                 )),
        //                 vec![]
        //             )),
        //             b("mail from:<me@there.net>\r\n")
        //         ),
        //         Command(
        //             Rcpt(SmtpPath::Relay(
        //                 vec![Domain("relay.net".to_string())],
        //                 SmtpAddress::Mailbox(
        //                     "him".to_string(),
        //                     Domain("unreachable.local".to_string()),
        //                 ),
        //             )),
        //             b("rcpt to:<@relay.net:him@unreachable.local>\r\n")
        //         ),
        //         Command(Quit, b("quit\r\n")),
        //     ]
        // );
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
