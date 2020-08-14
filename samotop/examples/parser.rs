//! Demonstrating the SMTP parser built from PEG grammar.

extern crate samotop;

use samotop::grammar::Parser;
use samotop::grammar::SmtpParser;

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
