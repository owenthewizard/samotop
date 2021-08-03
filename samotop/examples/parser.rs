//! Demonstrating the SMTP parser built from PEG grammar.

extern crate samotop;

use std::io::stdin;

use samotop::smtp::command::SmtpCommand;

use crate::samotop::smtp::*;

fn main() -> std::io::Result<()> {
    let mut input = String::new();

    let mut state = SmtpContext::default();

    loop {
        let _len = stdin().read_line(&mut input)?;

        while !input.is_empty() {
            let (i, item): (usize, SmtpCommand) =
                SmtpParser.parse(input.as_bytes(), &mut state).unwrap();
            println!("Parsed: {:#?}", item);
            input = input.split_off(i);
        }
    }
}
