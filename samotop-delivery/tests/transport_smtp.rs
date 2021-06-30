#[cfg(test)]
#[cfg(feature = "smtp-transport")]
mod test {
    use samotop_delivery::prelude::{ClientSecurity, Envelope, SmtpClient};

    #[async_attributes::test]
    #[ignore] // ignored as this needs a running server
    async fn smtp_transport_simple() {
        let envelope = Envelope::new(
            Some("user@localhost".parse().unwrap()),
            vec!["root@localhost".parse().unwrap()],
            "id".to_string(),
        )
        .unwrap();
        let message = "From: user@localhost\r\n\
                        Content-Type: text/plain\r\n\
                        \r\n\
                        Hello example"
            .as_bytes();
        println!("connecting");
        let client = SmtpClient::with_security("127.0.0.1:3025", ClientSecurity::None)
            .expect("should succeed");

        println!("sending");
        client.connect_and_send(envelope, message).await.unwrap();
    }
    #[test]
    fn smtp_transport_stream_is_sync() {
        fn is_sync<T: Sync>(_tested: T) {}

        let envelope = Envelope::new(
            Some("user@localhost".parse().unwrap()),
            vec!["root@localhost".parse().unwrap()],
            "id".to_string(),
        )
        .unwrap();

        println!("connecting");
        let client = SmtpClient::with_security("127.0.0.1:3025", ClientSecurity::None)
            .expect("should succeed");

        println!("sending");
        let stream = client.connect_and_send_stream(envelope);
        is_sync(stream);
    }
}
