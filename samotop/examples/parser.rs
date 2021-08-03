//! Demonstrating the SMTP parser built from PEG grammar.

extern crate samotop;

use std::io::stdin;

use samotop::smtp::command::SmtpCommand;

use crate::samotop::smtp::*;

fn main() -> std::io::Result<()> {
    let mut input = String::new();
    let mut state = SmtpContext::default();
    println!("Type some SMTP commands...");
    loop {
        let len = stdin().read_line(&mut input)?;
        if len != 0 {
            // fix LF to CRLF
            input = format!("{}\r\n", input.trim_end());
        }

        while !input.is_empty() {
            let (i, item): (usize, SmtpCommand) =
                match SmtpParser.parse(input.as_bytes(), &mut state) {
                    Ok(tuple) => tuple,
                    Err(e) => {
                        println!("Error: {}", e);
                        input = input.split_once('\n').expect("line").1.to_owned();
                        continue;
                    }
                };
            println!("Parsed: {:#?}", item);
            input = input.split_off(i);
        }
    }
}
