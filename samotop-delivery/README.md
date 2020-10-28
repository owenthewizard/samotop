# samotop-delivery 0.4.4-samotop-dev

samotop-delivery is an implementation of the smtp protocol client in Rust.
### Example

```rust
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

use samotop_delivery::prelude::{
    Envelope, SmtpClient, Transport,
};

async fn smtp_transport_simple() -> Result<()> {
    let envelope = Envelope::new(
            Some("user@localhost".parse().unwrap()),
            vec!["root@localhost".parse().unwrap()],
            "id".to_string(),
        ).unwrap();
    let message = "From: user@localhost\r\n\
                    Content-Type: text/plain\r\n\
                    \r\n\
                    Hello example"
                    .as_bytes();
    let client = SmtpClient::new("127.0.0.1:2525").unwrap();

    // Create a client, connect and send
    client.connect_and_send(envelope, message).await.unwrap();

    Ok(())
}
```

## Credits

This is a fork of [async-smtp](https://github.com/async-email/async-smtp/releases/tag/v0.3.4) 

## License - Apache-2.0/MIT

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
