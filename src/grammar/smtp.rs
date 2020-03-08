use bytes::Bytes;
use model::command::*;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;
use peg;

peg::parser!{
    pub grammar grammar() for str {
        pub rule session() -> Vec<SmtpInput>
            = WS()* i:script()
            { i }

        pub rule script() -> Vec<SmtpInput>
            = input()+

        pub rule input() -> SmtpInput
            = inp_none() / inp_command() / inp_invalid() / inp_incomplete()
        pub rule inp_command() -> SmtpInput
            = b:position!() c:command() e:position!()
            { SmtpInput::Command(b, e - b, c) }
        pub rule inp_none() -> SmtpInput
            = b:position!() s:$(NL() / WS()* NL()) e:position!()
            { SmtpInput::None(b, e - b, s.to_string()) }
        pub rule inp_invalid() -> SmtpInput
            = b:position!() s:$( str_invalid() ) e:position!()
            { SmtpInput::Invalid(b, e - b, Bytes::from(s)) }
        pub rule inp_incomplete() -> SmtpInput
            = b:position!() s:$( str_incomplete() ) e:position!()
            { SmtpInput::Incomplete(b, e - b, Bytes::from(s)) }

        rule str_invalid() = quiet!{ "\n" / (!['\n'][_]) + "\n" } / expected!("invalid input")
            rule str_incomplete() = quiet!{ [_]+ } / expected!("incomplete input")

            // https://github.com/kevinmehall/rust-peg/issues/216
            rule i(literal: &'static str) = input:$([_]*<{literal.len()}>)
            {? if input.eq_ignore_ascii_case(literal) { Ok(()) } else { Err(literal) } }

        pub rule command() -> SmtpCommand
            = cmd_starttls() /
            cmd_helo() /
            cmd_ehlo() /
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
            = i("mail from:") p:path_reverse() NL()
            { SmtpCommand::Mail(SmtpMail::Mail(p)) }
        pub rule cmd_send() -> SmtpCommand
            = i("send from:") p:path_reverse() NL()
            { SmtpCommand::Mail(SmtpMail::Send(p)) }
        pub rule cmd_soml() -> SmtpCommand
            = i("soml from:") p:path_reverse() NL()
            { SmtpCommand::Mail(SmtpMail::Soml(p)) }
        pub rule cmd_saml() -> SmtpCommand
            = i("saml from:") p:path_reverse() NL()
            { SmtpCommand::Mail(SmtpMail::Saml(p)) }

        pub rule cmd_rcpt() -> SmtpCommand
            = i("rcpt to:") p:path_forward() NL()
            { SmtpCommand::Rcpt(p) }

        pub rule cmd_helo() -> SmtpCommand
            = i("helo") SP() h:host() NL()
            { SmtpCommand::Helo(SmtpHelo::Helo(h)) }

        pub rule cmd_ehlo() -> SmtpCommand
            = i("ehlo") SP() h:host() NL()
            { SmtpCommand::Helo(SmtpHelo::Ehlo(h)) }

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
            = s:str() "@" h:host()
            { SmtpAddress::Mailbox (s, h) }

        rule athost() -> SmtpHost
            = "@" h:host() (&",@" "," / ":")
            { h }

        rule strparam() -> String
            = SP() s:str()
            { s }

        pub rule host() -> SmtpHost
            = host_numeric() /
            host_ipv4() /
            host_ipv6() /
            host_other() /
            host_domain()

            rule host_domain() -> SmtpHost
            = s:$( label() ("." label())* )
            { SmtpHost::Domain(s.to_string()) }
        rule domain() = quiet!{label() ("." label())*} / expected!("domain name")
            rule label() = ['a'..='z' | 'A'..='Z' | '0'..='9'] ['-' | 'a'..='z' | 'A'..='Z' | '0'..='9']*

            rule host_numeric() -> SmtpHost
            = "#" s:$(['0'..='9']+ / expected!("ipv4 number"))
            { match u32::from_str(s) {
                                         Ok(ip) => SmtpHost::Ipv4(Ipv4Addr::from(ip)),
                                         Err(e) => SmtpHost::Invalid{label:"numeric".to_string(), literal:s.to_string()},
                                     } }

        rule host_ipv4() -> SmtpHost
            = "[" s:$(ipv4addr()) "]"
            { match Ipv4Addr::from_str(s) {
                                              Ok(ip) => SmtpHost::Ipv4(ip),
                                              Err(e) => SmtpHost::Invalid{label:"ipv4".to_string(), literal:s.to_string()},
                                          } }
        rule ipv4addr() = quiet!{ipv4part() "." ipv4part() "." ipv4part() "." ipv4part()} / expected!("ipv4 address")
        rule ipv4part() = "25" ['0'..='5'] /
            "2" ['0'..='4'] ['0'..='9'] /
            ['0'..='1'] ['0'..='9'] ['0'..='9']? /
            ['0'..='9'] ['0'..='9']?

                rule host_ipv6() -> SmtpHost
                    = l:$(i("IPv6")) ":" s:$(ipv6addr())
                    { match Ipv6Addr::from_str(s) {
                                                      Ok(ip) => SmtpHost::Ipv6(ip),
                                                      Err(e) => SmtpHost::Invalid{label:l.to_string(), literal:s.to_string()},
                                                  } }
        rule ipv6addr() = quiet!{['0'..='9' | 'a'..='f' | 'A'..='F' | ':' | '.']+} / expected!("ipv6 address")

        rule host_other() -> SmtpHost
            = l:str() ":" s:str()
            { SmtpHost::Other{label:l, literal:s} }

        pub rule str() -> String
            = str_quoted() / str_plain()
            rule str_plain() -> String
            = v:char()*
            { v.iter().fold(String::new(), |s, c| s + c) }
        rule str_quoted() -> String
            = ['"'] v:qchar()* ['"']
            { v.iter().fold(String::new(), |s, c| s + c) }
        rule qchar() -> &'input str
            = qchar_regular() / char_special()
            rule qchar_regular() -> &'input str
            = s:$(quiet!{!(['"' | '\\'] / "\r" / "\n") [_]} / expected!("quoted character"))
            { s }
        rule char() -> &'input str
            = char_regular() / char_special()
            rule char_regular() -> &'input str
            = s:$(quiet!{['-' | '!' | '#' | '$' | '%' | '&' |
                '\'' | '*' | '+' | '-' | '`' | '/' |
                    '0'..='9' | 'a'..='z' | 'A'..='Z' |
                    '=' | '?' | '~' | '^' | '_' | '{' | '}' | '|'
            ]} / expected!("regular character"))
            { s }
        rule char_special() -> &'input str
            = "\\" s:$(quiet!{[_]} / expected!("special character"))
            { s }
        rule NL() = quiet!{"\r\n"} / quiet!{"\n"} / expected!("{NL}")
            rule SP() = quiet!{" "} / expected!("{SP}")
            rule WS() = quiet!{SP() / "\t"} / expected!("{WS}")
    }
}

#[cfg(test)]
mod tests {
    use super::grammar::{host, script, session, command};
    use bytes::Bytes;
    use model::command::SmtpCommand::*;
    use model::command::SmtpHost::*;
    use model::command::SmtpInput::Invalid;
    use model::command::SmtpInput::*;
    use model::command::*;

    #[test]
    fn script_parses_unknown_command() {
        let result = script("sOmE other command\r\n").unwrap();
        assert_eq!(
            result,
            vec![SmtpInput::Invalid(
                0,
                20,
                Bytes::from("sOmE other command\r\n"),
            )]
        );
    }

    #[test]
    fn host_parses_unknown_host() {
        let result = host("who:what").unwrap();
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
        let result = command("STARTTLS\r\n").unwrap();
        assert_eq!(result, SmtpCommand::StartTls);
    }

    #[test]
    fn script_parses_whitespace_line() {
        let result = script("   \r\n\t\t\r\n").unwrap();
        assert_eq!(
            result,
            vec![
            SmtpInput::None(0, 5, "   \r\n".to_string()),
            SmtpInput::None(5, 4, "\t\t\r\n".to_string()),
            ]
        );
    }

    #[test]
    fn session_parses_helo() {
        let result = session("helo domain.com\r\n").unwrap();

        assert_eq!(
            result,
            vec![Command(
                0,
                17,
                Helo(SmtpHelo::Helo(Domain("domain.com".to_string()))),
            )]
        );
    }

    #[test]
    fn session_parses_data() {
        let result = session("DATA\r\n ěšě\r\nš\nčš").unwrap();

        assert_eq!(
            result,
            vec![
            Command(0, 6, Data),
            Invalid(6, 9, Bytes::from(" ěšě\r\n")),
            Invalid(15, 3, Bytes::from("š\n")),
            Incomplete(18, 4, Bytes::from("čš")),
            ]
        );
    }

    #[test]
    fn session_parses_wrong_newline() {
        let result = session("QUIT\nQUIT\r\nquit\r\n").unwrap();

        assert_eq!(
            result,
            vec![
            Command(0, 5, Quit),
            Command(5, 6, Quit),
            Command(11, 6, Quit),
            ]
        );
    }

    #[test]
    fn session_parses_incomplete_command() {
        let result = session("QUIT\r\nQUI").unwrap();

        assert_eq!(
            result,
            vec![Command(0, 6, Quit), Incomplete(6, 3, Bytes::from("QUI"))]
        );
    }

    #[test]
    fn session_parses_helo_mail_rcpt_quit() {
        let result = session(concat!(
                "helo domain.com\r\n",
                "mail from:<me@there.net>\r\n",
                "rcpt to:<@relay.net:him@unreachable.local>\r\n",
                "quit\r\n"
        )).unwrap();

        assert_eq!(
            result,
            vec![
            Command(
                0,
                17,
                Helo(SmtpHelo::Helo(Domain("domain.com".to_string()))),
            ),
            Command(
                17,
                26,
                Mail(SmtpMail::Mail(SmtpPath::Direct(SmtpAddress::Mailbox(
                                "me".to_string(),
                                Domain("there.net".to_string()),
                )))),
            ),
            Command(
                43,
                44,
                Rcpt(SmtpPath::Relay(
                        vec![Domain("relay.net".to_string())],
                        SmtpAddress::Mailbox(
                            "him".to_string(),
                            Domain("unreachable.local".to_string()),
                        ),
                )),
            ),
            Command(87, 6, Quit),
            ]
                );
    }
}
