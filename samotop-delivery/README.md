# samotop-delivery 0.5.0

samotop-delivery is a set of transports to deliver mail to,
notably to SMTP/LMTP, but also maildir... It is used in Samotop
as a delivery solution for incoming mail.

### Example

```rust
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

use samotop_delivery::prelude::{
    Envelope, SmtpClient, Transport,
};

async fn smtp_transport_simple() -> Result<()> {
    let envelope = Envelope::new(
            Some("user@localhost".parse()?),
            vec!["root@localhost".parse()?],
            "id".to_string(),
        )?;
    let message = "From: user@localhost\r\n\
                    Content-Type: text/plain\r\n\
                    \r\n\
                    Hello example"
                    .as_bytes();
    let client = SmtpClient::new("127.0.0.1:2525")?;

    // Create a client, connect and send
    client.connect_and_send(envelope, message).await?;

    Ok(())
}
```

## Credits

This is a fork of [async-smtp](https://github.com/async-email/async-smtp/releases/tag/v0.3.4) 

## License - MIT OR Apache-2.0

<sup>
Licensed under either of <a href="../LICENSE-Apache2">Apache License, Version
2.0</a> or <a href="../LICENSE">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
