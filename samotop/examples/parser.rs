//! Demonstrating the SMTP parser built from PEG grammar.

#[cfg(any(feature = "parser-peg", feature = "parser-nom"))]
fn main() -> std::io::Result<()> {
    use samotop::smtp::command::SmtpCommand;
    use samotop::smtp::*;
    use std::io::stdin;

    let mut input = String::new();
    let state = SmtpContext::default();
    println!("Type some SMTP commands...");
    loop {
        let len = stdin().read_line(&mut input)?;
        if len != 0 {
            // fix LF to CRLF
            input = format!("{}\r\n", input.trim_end());
        }

        while !input.is_empty() {
            let (i, item): (usize, SmtpCommand) = match SmtpParser.parse(input.as_bytes(), &state) {
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

#[cfg(not(any(feature = "parser-peg", feature = "parser-nom")))]
fn main() -> std::io::Result<()> {
    panic!("This will only work with either parser-peg or parser-nom feature enabled.")
}
