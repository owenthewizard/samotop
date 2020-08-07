//! Demonstrating the SMTP parser built from PEG grammar.

extern crate async_std;
extern crate env_logger;
extern crate samotop;

use samotop::grammar::Parser;
use samotop::grammar::SmtpParser;

fn main() {
    let input = String::new()
        + "HELO there\r\n"
        + "MAIL FROM:<a@b.c>\r\n"
        + "RCPT TO:<x@y.z>\r\n"
        + "DATA\r\n"
        + "QUIT\r\n";
    let result = SmtpParser.script(input.as_bytes()).unwrap();
    for r in result {
        println!("Parsed: {:?}", r);
    }
}
