//! Demonstrating the SMTP parser built from PEG grammar.

extern crate samotop;

use samotop::smtp::command::SmtpCommand;

use crate::samotop::smtp::*;

fn main() {
    let input = String::new()
        + "EHLO there\r\n"
        + "MAIL FROM:<a@b.c> param1=value1 param2=value2\r\n"
        + "RCPT TO:<x@y.z>\r\n"
        + "DATA\r\n"
        + "QUIT\r\n";

    let mut input = input.as_bytes();
    let mut state = SmtpState::default();

    while !input.is_empty() {
        let (i, item): (usize, SmtpCommand) = SmtpParser.parse(input, &mut state).unwrap();
        input = &input[i..];
        println!("Parsed: {:#?}", item);
    }
}
