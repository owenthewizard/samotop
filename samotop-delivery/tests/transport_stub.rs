#[cfg(test)]
#[cfg(feature = "smtp-transport")]
mod test {
    use samotop_delivery::prelude::{EmailAddress, Envelope, Transport};
    use samotop_delivery::stub::StubTransport;

    #[async_attributes::test]
    async fn stub_transport() {
        let sender_ok = StubTransport::new_positive();
        let sender_ko = StubTransport::new(Err("fail".into()));
        let envelope = Envelope::new(
            Some(EmailAddress::new("user@localhost".to_string()).unwrap()),
            vec![EmailAddress::new("root@localhost".to_string()).unwrap()],
            "id".to_string(),
        )
        .unwrap();
        let message = "Hello ß☺ example".as_bytes();
        sender_ok.send(envelope.clone(), message).await.unwrap();
        sender_ko.send(envelope.clone(), message).await.unwrap_err();
    }

    #[test]
    fn stub_transport_stream_is_sync() {
        fn is_sync<T: Sync>(_tested: T) {}

        let envelope = Envelope::new(
            Some("user@localhost".parse().unwrap()),
            vec!["root@localhost".parse().unwrap()],
            "id".to_string(),
        )
        .unwrap();

        println!("connecting");
        let transport = StubTransport::new_positive();
        //is_sync(transport);

        println!("sending");
        let stream = transport.send_stream(envelope);
        is_sync(stream);
    }
}
