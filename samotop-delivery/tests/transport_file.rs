#[cfg(test)]
#[cfg(feature = "file-transport")]
mod test {
    use samotop_delivery::file::FileTransport;
    use samotop_delivery::prelude::{Envelope, Transport};

    use std::env::temp_dir;
    use std::fs::remove_file;
    use std::fs::File;
    use std::io::Read;

    #[async_attributes::test]
    async fn file_transport() {
        let sender = FileTransport::new(temp_dir());
        let envelope = Envelope::new(
            Some("user@localhost".parse().unwrap()),
            vec!["root@localhost".parse().unwrap()],
            "id".to_string(),
        )
        .unwrap();
        let message = "Hello ß☺ example".as_bytes();

        let result = sender.send(envelope, message).await;
        assert!(result.is_ok());

        let file = format!("{}/{}.json", temp_dir().to_str().unwrap(), "id");
        let mut f = File::open(file.clone()).unwrap();
        let mut buffer = String::new();
        let _ = f.read_to_string(&mut buffer);

        assert_eq!(
            buffer,
            "{\"envelope\":{\"forward_path\":[\"root@localhost\"],\"reverse_path\":\"user@localhost\",\"message_id\":\"id\"}}\nHello ß☺ example"
        );

        remove_file(file).unwrap();
    }
}
