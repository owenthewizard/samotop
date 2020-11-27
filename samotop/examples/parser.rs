//! Demonstrating the SMTP parser built from PEG grammar.

extern crate samotop;

use crate::samotop::parser::*;

fn main() {
    let input = String::new()
        + "EHLO there\r\n"
        + "MAIL FROM:<a@b.c> param1=value1 param2=value2\r\n"
        + "RCPT TO:<x@y.z>\r\n"
        + "DATA\r\n"
        + "QUIT\r\n";

    let mut input = input.as_bytes();

    while !input.is_empty() {
        let (i, item) = SmtpParser::default().parse_command(input).unwrap();
        input = i;
        println!("Parsed: {:#?}", item);
    }
}
