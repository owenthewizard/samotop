//! Demonstrating the SMTP parser built from PEG grammar.

extern crate samotop_core;
extern crate samotop_parser;

use samotop_core::service::parser::Parser;
use samotop_parser::SmtpParser;

fn main() {
    let input = String::new()
        + "EHLO there\r\n"
        + "MAIL FROM:<a@b.c> param1=value1 param2=value2\r\n"
        + "RCPT TO:<x@y.z>\r\n"
        + "DATA\r\n"
        + "QUIT\r\n";
    let result = SmtpParser.script(input.as_bytes()).unwrap();

    println!("Parsed: {:#?}", result);
}
