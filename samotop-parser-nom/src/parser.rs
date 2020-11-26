/*
    Aim: wrap generated parser fns in struct
*/

use nom::{
    branch::alt,
    bytes::streaming::tag_no_case,
    bytes::streaming::take_until,
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
    smtp::{ReadControl, SmtpAddress, SmtpCommand, SmtpHelo, SmtpHost, SmtpPath},
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
        use rustyknife::rfc5321::Command as C;
        use rustyknife::types::{AddressLiteral, DomainPart};
        use std::net::IpAddr;
        match rustyknife::rfc5321::command::<rustyknife::behaviour::Intl>(input)? {
            (i, cmd) => Ok(match cmd {
                C::HELO(domain) => {
                    SmtpCommand::Helo(SmtpHelo::Ehlo(SmtpHost::Domain(domain.to_string())))
                }
                C::EHLO(DomainPart::Domain(domain)) => {
                    SmtpCommand::Helo(SmtpHelo::Ehlo(SmtpHost::Domain(domain.to_string())))
                }
                C::EHLO(DomainPart::Address(AddressLiteral::IP(IpAddr::V4(ip)))) => {
                    SmtpCommand::Helo(SmtpHelo::Ehlo(SmtpHost::Ipv4(ip)))
                }
                C::EHLO(DomainPart::Address(AddressLiteral::IP(IpAddr::V6(ip)))) => {
                    SmtpCommand::Helo(SmtpHelo::Ehlo(SmtpHost::Ipv6(ip)))
                }
                C::EHLO(DomainPart::Address(AddressLiteral::Tagged(label, literal))) => {
                    SmtpCommand::Helo(SmtpHelo::Ehlo(SmtpHost::Other { label, literal }))
                }
                C::EHLO(DomainPart::Address(AddressLiteral::FreeForm(literal))) => {
                    SmtpCommand::Helo(SmtpHelo::Ehlo(SmtpHost::Invalid {
                        label: String::new(),
                        literal,
                    }))
                }
                C::MAIL(reverse_path, params) => {
                    SmtpCommand::Mail(SmtpMail::Mail())
                }
                C::RCPT(forward_path, params) => {
                    todo!()
                }
                C::DATA => SmtpCommand::Data,
                C::RSET => SmtpCommand::Rset,
                C::NOOP(param) => {
                    SmtpCommand::Noop(param.map(|s| vec![s.to_string()]).unwrap_or_default())
                }
                C::QUIT => SmtpCommand::Quit,
                C::VRFY(param) => SmtpCommand::Vrfy(param.to_string()),
                C::EXPN(param) => SmtpCommand::Expn(param.to_string()),
                C::HELP(param) => {
                    SmtpCommand::Help(param.map(|s| vec![s.to_string()]).unwrap_or_default())
                }
            }),
        }
    }
    fn script(&self, input: &[u8]) -> Result<Vec<ReadControl>> {
        unimplemented!()
    }
    fn forward_path(&self, input: &[u8]) -> Result<SmtpPath> {
        unimplemented!()
    }
}

#[cfg(test)]
mod cmd_tests {
    use super::*;

    #[test]
    fn test_quit() {
        let res = smtp_cmd(b"QUIT\r\n".as_ref()).unwrap();
        assert_eq!(res.1, ("QUIT", ""));
        assert_eq!(res.0, b"\r\n");
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
