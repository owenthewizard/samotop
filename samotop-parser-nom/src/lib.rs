/*!
Aim: wrap rustyknife nom parser for samotop

:warning: this brings **GPLv3** requirements of the rustyknife crate.

*/

use nom::{bytes::streaming::tag, Err};
use rustyknife::{
    rfc5321::mailbox,
    rfc5321::Command,
    rfc5321::ReversePath,
    rfc5321::{ForwardPath, Path},
    types::AddressLiteral,
    types::DomainPart,
};
use samotop_core::smtp::{command::*, *};
pub use samotop_core::smtp::{ParseError, ParseResult, Parser};
use std::net::IpAddr;

#[derive(Clone, Copy, Debug, Default)]
pub struct SmtpParserNom;

impl Parser<SmtpCommand> for SmtpParserNom {
    fn parse(&self, input: &[u8], state: &SmtpState) -> ParseResult<SmtpCommand> {
        if input.is_empty() {
            return Err(ParseError::Incomplete);
        }
        if let Some(mode) = state.transaction.mode {
            return Err(ParseError::Mismatch(format!(
                "Not parsing in {:?} mode",
                mode
            )));
        }
        match rustyknife::rfc5321::command::<rustyknife::behaviour::Intl>(input) {
            Ok((i, cmd)) => Ok((i.len(), map_cmd(cmd))),
            Err(e) => Err(map_error(e)),
        }
    }
}

impl SmtpParserNom {
    pub fn forward_path<'i>(&self, input: &'i [u8]) -> ParseResult<SmtpPath> {
        let len = input.len();
        let (input, _) = tag("<")(input).map_err(map_error)?;
        let (input, m) = mailbox::<rustyknife::behaviour::Intl>(input).map_err(map_error)?;
        let (input, _) = tag(">")(input).map_err(map_error)?;
        Ok((len - input.len(), map_path(Path(m, vec![]))))
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
        Command::HELO(domain) => SmtpCommand::Helo(SmtpHelo {
            verb: "HELO".to_owned(),
            host: SmtpHost::Domain(domain.to_string()),
        }),
        Command::EHLO(host) => SmtpCommand::Helo(SmtpHelo {
            verb: "EHLO".to_owned(),
            host: map_host(host),
        }),
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
        ForwardPath::PostMaster(Some(domain)) => SmtpPath::Mailbox {
            name: "postmaster".to_string(),
            host: SmtpHost::Domain(domain.to_string()),
            relays: vec![],
        },
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
    SmtpPath::Mailbox {
        name: local.to_string(),
        host: map_host(domain),
        relays: domains
            .into_iter()
            .map(|d| SmtpHost::Domain(d.to_string()))
            .collect(),
    }
}
fn map_host(host: DomainPart) -> SmtpHost {
    match host {
        DomainPart::Domain(domain) => SmtpHost::Domain(domain.to_string()),
        DomainPart::Address(AddressLiteral::IP(IpAddr::V4(ip))) => SmtpHost::Ipv4(ip),
        DomainPart::Address(AddressLiteral::IP(IpAddr::V6(ip))) => SmtpHost::Ipv6(ip),
        DomainPart::Address(AddressLiteral::Tagged(label, literal)) => {
            SmtpHost::Other { label, literal }
        }
        DomainPart::Address(AddressLiteral::FreeForm(literal)) => SmtpHost::Invalid {
            label: String::new(),
            literal,
        },
    }
}
