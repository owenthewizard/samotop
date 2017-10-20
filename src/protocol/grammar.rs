use std::str::FromStr;
use std::net::{Ipv4Addr, Ipv6Addr};
use model::request::*;

include!(concat!(env!("OUT_DIR"), "/grammar.rs"));

#[cfg(test)]
mod tests {
    use protocol::grammar::*;
    use model::request::SmtpHost::*;
    use model::request::SmtpCommand::*;

    #[test]
    fn script_parses_unknown_command() {
        let result = script("sOmE other command\r\n").unwrap();
        assert_eq!(
            result,
            vec![
                SmtpInput::Command(
                    0,
                    20,
                    SmtpCommand::Unknown("sOmE other command\r\n".to_string())
                ),
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
                SmtpInput::Command(0, 17, Helo(Domain("domain.com".to_string()))),
            ]
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
                SmtpInput::Command(0, 17, Helo(Domain("domain.com".to_string()))),
                SmtpInput::Command(
                    17,
                    26,
                    Mail(
                        SmtpDelivery::Mail,
                        SmtpPath::Direct(SmtpAddress::Mailbox(
                            "me".to_string(),
                            SmtpHost::Domain("there.net".to_string()),
                        )),
                    )
                ),
                SmtpInput::Command(
                    43,
                    44,
                    Rcpt(SmtpPath::Relay(
                        vec![Domain("relay.net".to_string())],
                        SmtpAddress::Mailbox(
                            "him".to_string(),
                            SmtpHost::Domain("unreachable.local".to_string()),
                        ),
                    ))
                ),
                SmtpInput::Command(87, 6, Quit),
            ]
        );
    }
}
