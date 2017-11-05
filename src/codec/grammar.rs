use std::str::FromStr;
use std::net::{Ipv4Addr, Ipv6Addr};
use bytes::Bytes;
use model::request::*;

include!(concat!(env!("OUT_DIR"), "/grammar.rs"));

#[cfg(test)]
mod tests {
    use codec::grammar::*;
    use model::request::SmtpHost::*;
    use model::request::SmtpCommand::*;
    use model::request::SmtpInput::*;
    use model::request::SmtpInput::Invalid;

    #[test]
    fn script_parses_unknown_command() {
        let result = script("sOmE other command\r\n").unwrap();
        assert_eq!(
            result,
            vec![
                SmtpInput::Invalid(0, 20, Bytes::from("sOmE other command\r\n")),
            ]
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
            vec![
                Command(
                    0,
                    17,
                    Helo(SmtpHelo::Helo(Domain("domain.com".to_string())))
                ),
            ]
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
                Invalid(0, 5, Bytes::from("QUIT\n")),
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
                    Helo(SmtpHelo::Helo(Domain("domain.com".to_string())))
                ),
                Command(
                    17,
                    26,
                    Mail(SmtpMail::Mail(SmtpPath::Direct(SmtpAddress::Mailbox(
                        "me".to_string(),
                        Domain("there.net".to_string()),
                    ))))
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
                    ))
                ),
                Command(87, 6, Quit),
            ]
        );
    }
}
