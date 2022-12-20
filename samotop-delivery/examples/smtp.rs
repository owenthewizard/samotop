use async_std::{io, io::Read, task};
use clap::Parser;
use samotop_delivery::prelude::{EmailAddress, Envelope, SmtpClient};

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

fn main() {
    env_logger::init();
    println!("This example takes e-mail body from the stdin. Ctrl+D usually closes interactive input on linux shells, Ctrl+Z on Windows cmd. Run it with RUST_LOG=debug for detailed feedback.");

    // Collect all inputs
    let opt = Opt::parse();
    let id = "some_random_id";

    // Send mail
    let result = task::block_on(send_mail(opt, id, io::stdin()));

    match result {
        Ok(message) => println!("Email sent: {}", message),
        Err(e) => println!("Could not send email: {:?}", e),
    }
}

async fn send_mail<R>(opt: Opt, id: &str, mail_body: R) -> Result<String>
where
    R: Read + Send + Sync + Unpin + 'static,
{
    // Compose a mail
    let envelope = Envelope::new(Some(opt.from), opt.to, id.to_string())?;

    // Open an SMTP connection to given address and send the mail
    let response = SmtpClient::new(opt.server)?
        .connect_and_send(envelope, mail_body)
        .await?;

    Ok(response.message.join(" "))
}

#[derive(Parser, Debug)]
#[command(name = "smtp")]
struct Opt {
    /// Mail from
    #[arg(short = 'f', name = "sender address")]
    from: EmailAddress,

    /// Rcpt to, can be repeated multiple times
    #[arg(
        short = 't',
        name = "recipient address",
        required = true,
        num_args = 1..
    )]
    to: Vec<EmailAddress>,

    /// SMTP server address:port to talk to
    #[arg(short = 's', name = "smtp server", default_value = "localhost:25")]
    server: String,
}
