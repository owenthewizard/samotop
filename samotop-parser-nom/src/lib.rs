/*
    Aim: wrap rustyknife nom parser for samotop
*/

// use nom::{
//     branch::alt,
//     bytes::streaming::tag_no_case,
//     bytes::streaming::take_until,
//     bytes::streaming::{escaped_transform, is_a, is_not, tag, take, take_while_m_n},
//     character::streaming::{alphanumeric0, alphanumeric1},
//     combinator::map,
//     combinator::opt,
//     combinator::recognize,
//     multi::{many0, many1},
//     sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
//     IResult,
// };
use nom::{bytes::streaming::tag, Err};
use rustyknife::{
    rfc5321::mailbox,
    rfc5321::Command,
    rfc5321::ReversePath,
    rfc5321::{ForwardPath, Path},
    types::AddressLiteral,
    types::DomainPart,
};
use samotop_model::{
    mail::MailSetup,
    parser::{ParseError, ParseResult, Parser},
    smtp::*,
};
use std::net::IpAddr;

#[derive(Clone, Copy, Debug, Default)]
pub struct SmtpParserNom;

impl MailSetup for SmtpParserNom {
    fn setup(self, builder: &mut samotop_model::mail::Builder) {
        builder.parser.insert(0, Box::new(self))
    }
}

impl Parser for SmtpParserNom {
    fn parse_command<'i>(&self, input: &'i [u8]) -> ParseResult<'i, Box<dyn SmtpSessionCommand>> {
        match rustyknife::rfc5321::command::<rustyknife::behaviour::Intl>(input) {
            Ok((i, cmd)) => Ok((i, Box::new(map_cmd(cmd)))),
            Err(e) => Err(map_error(e)),
        }
    }
}

impl SmtpParserNom {
    pub fn forward_path<'i>(&self, input: &'i [u8]) -> ParseResult<'i, SmtpPath> {
        let (input, _) = tag("<")(input).map_err(map_error)?;
        let (input, m) = mailbox::<rustyknife::behaviour::Intl>(input).map_err(map_error)?;
        let (input, _) = tag(">")(input).map_err(map_error)?;
        Ok((input, map_path(Path(m, vec![]))))
    }
}

fn map_error(e: Err<()>) -> ParseError {
    match e {
        Err::Incomplete(_) => ParseError::Incomplete,
        Err::Error(()) => ParseError::Mismatch("nom recoverable error".into()),
        Err::Failure(()) => ParseError::Failed("nom failure".into()),
    }
}
fn map_cmd(cmd: Command) -> SmtpCommand {
    match cmd {
        Command::HELO(domain) => {
            SmtpCommand::Helo(SmtpHelo::Ehlo(SmtpHost::Domain(domain.to_string())))
        }
        Command::EHLO(host) => SmtpCommand::Helo(SmtpHelo::Ehlo(map_host(host))),
        Command::MAIL(path, params) => SmtpCommand::Mail(SmtpMail::Mail(
            map_reverse_path(path),
            params.into_iter().map(|p| p.to_string()).collect(),
        )),
        Command::RCPT(path, params) => SmtpCommand::Rcpt(SmtpRcpt(
            map_forward_path(path),
            params.into_iter().map(|p| p.to_string()).collect(),
        )),
        Command::DATA => SmtpCommand::Data,
        Command::RSET => SmtpCommand::Rset,
        Command::NOOP(param) => {
            SmtpCommand::Noop(param.map(|s| vec![s.to_string()]).unwrap_or_default())
        }
        Command::QUIT => SmtpCommand::Quit,
        Command::VRFY(param) => SmtpCommand::Vrfy(param.to_string()),
        Command::EXPN(param) => SmtpCommand::Expn(param.to_string()),
        Command::HELP(param) => {
            SmtpCommand::Help(param.map(|s| vec![s.to_string()]).unwrap_or_default())
        }
    }
}
fn map_forward_path(path: ForwardPath) -> SmtpPath {
    match path {
        ForwardPath::Path(path) => map_path(path),
        ForwardPath::PostMaster(None) => SmtpPath::Postmaster,
        ForwardPath::PostMaster(Some(domain)) => SmtpPath::Direct(SmtpAddress::Mailbox(
            "postmaster".to_string(),
            SmtpHost::Domain(domain.to_string()),
        )),
    }
}
fn map_reverse_path(path: ReversePath) -> SmtpPath {
    match path {
        ReversePath::Path(path) => map_path(path),
        ReversePath::Null => SmtpPath::Null,
    }
}
fn map_path(path: Path) -> SmtpPath {
    let Path(mailbox, domains) = path;
    let (local, domain) = mailbox.into_parts();
    SmtpPath::Relay(
        domains
            .into_iter()
            .map(|d| SmtpHost::Domain(d.to_string()))
            .collect(),
        SmtpAddress::Mailbox(local.to_string(), map_host(domain)),
    )
}
fn map_host(host: DomainPart) -> SmtpHost {
    match host {
        DomainPart::Domain(domain) => SmtpHost::Domain(domain.to_string()),
        DomainPart::Address(AddressLiteral::IP(IpAddr::V4(ip))) => SmtpHost::Ipv4(ip.clone()),
        DomainPart::Address(AddressLiteral::IP(IpAddr::V6(ip))) => SmtpHost::Ipv6(ip.clone()),
        DomainPart::Address(AddressLiteral::Tagged(label, literal)) => SmtpHost::Other {
            label: label.clone(),
            literal: literal.clone(),
        },
        DomainPart::Address(AddressLiteral::FreeForm(literal)) => SmtpHost::Invalid {
            label: String::new(),
            literal: literal.clone(),
        },
    }
}
