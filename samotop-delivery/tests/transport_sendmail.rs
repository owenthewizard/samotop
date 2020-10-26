#[cfg(test)]
#[cfg(feature = "sendmail-transport")]
mod test {
    use samotop_delivery::prelude::{Envelope, Transport};
    use samotop_delivery::sendmail::SendmailTransport;

    #[async_attributes::test]
    async fn sendmail_transport_simple() {
        let sender = SendmailTransport::new();
        let envelope = Envelope::new(
            Some("user@localhost".parse().unwrap()),
            vec!["root@localhost".parse().unwrap()],
            "id".to_string(),
        )
        .unwrap();
        let message = "Hello ß☺ example".as_bytes();

        let result = sender.send(envelope, message).await;
        println!("{:?}", result);
        assert!(result.is_ok());
    }
}
