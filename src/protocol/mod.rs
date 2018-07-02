mod line;
mod smtp;
pub use self::line::*;
pub use self::smtp::*;

#[cfg(test)]
mod tests {
    //use env_logger;
    use bytes;
    use model::controll::ClientControll;
    use protocol::SmtpCodec;
    use tokio_codec::Encoder;

    #[test]
    fn decode_takes_first_line() {
        let mut sut = SmtpCodec::new();

        let mut buf = bytes::BytesMut::from(&b"helo there\r\nquit\r\n"[..]);

        let result = sut.decode_either(&mut buf).unwrap().unwrap();

        assert_eq!(format!("{:?}", result), "Command(\"helo there\\r\\n\")");
        assert_eq!(buf.len(), 6); // only quit\r\n is left
    }

    #[test]
    fn decode_checks_sanity() {
        let mut sut = SmtpCodec::new();

        let mut buf = bytes::BytesMut::from(&b"he\r\n"[..]);

        let result = sut.decode_either(&mut buf).unwrap().unwrap();

        assert_eq!(format!("{:?}", result), "Invalid(b\"he\\r\\n\")");
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn decode_recovers_from_errors() {
        let mut sut = SmtpCodec::new();

        let mut buf = bytes::BytesMut::from(&b"!@#\r\nquit\r\n"[..]);

        let _ = sut.decode_either(&mut buf).unwrap().unwrap();
        let result = sut.decode_either(&mut buf).unwrap().unwrap();

        assert_eq!(format!("{:?}", result), "Command(\"quit\\r\\n\")");
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn decode_takes_second_line() {
        let mut sut = SmtpCodec::new();

        let mut buf = bytes::BytesMut::from(&b"helo there\r\nquit\r\n"[..]);

        sut.decode_either(&mut buf).expect("ok");
        let result = sut.decode_either(&mut buf).unwrap().unwrap();

        assert_eq!(format!("{:?}", result), "Command(\"quit\\r\\n\")");
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn decode_handles_empty_data_buffer() {
        let mut sut = SmtpCodec::new();

        let mut buf = bytes::BytesMut::from(&b"data\r\n"[..]);

        let result = sut.decode_either(&mut buf).unwrap().unwrap();
        assert_eq!(format!("{:?}", result), "Command(\"data\\r\\n\")");

        let result = sut.decode_either(&mut buf).unwrap();
        assert_eq!(format!("{:?}", result), "None");
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn decode_finds_data_dot() {
        let mut sut = SmtpCodec::new();

        let mut buf = bytes::BytesMut::new();
        sut.encode(ClientControll::AcceptData, &mut buf).unwrap();

        buf.extend_from_slice(&b"daaataaa\r\n"[..]);

        let result = sut.decode_either(&mut buf).unwrap().unwrap();
        assert_eq!(format!("{:?}", result), "Data(b\"daaataaa\")");

        buf.extend_from_slice(&b".\r\n"[..]);

        let result = sut.decode_either(&mut buf).unwrap().unwrap();
        assert_eq!(format!("{:?}", result), "DataEnd(b\"\\r\\n.\\r\\n\")");

        let result = sut.decode_either(&mut buf).unwrap();
        assert_eq!(format!("{:?}", result), "None");

        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn decode_finds_data_dot_after_empty_data() {
        //env_logger::init();
        let mut sut = SmtpCodec::new();

        let mut buf = bytes::BytesMut::new();
        sut.encode(ClientControll::AcceptData, &mut buf).unwrap();

        buf.extend_from_slice(&b".\r\n"[..]);

        let result = sut.decode_either(&mut buf).unwrap().unwrap();
        assert_eq!(format!("{:?}", result), "DataEnd(b\".\\r\\n\")");

        let result = sut.decode_either(&mut buf).unwrap();
        assert_eq!(format!("{:?}", result), "None");

        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn decode_handles_dangling_data() {
        let mut sut = SmtpCodec::new();

        let mut buf = bytes::BytesMut::from(&b"helo "[..]);

        let result = sut.decode_either(&mut buf).unwrap();
        assert!(result.is_none());

        buf.extend_from_slice(&b"there\r\nxx"[..]);

        let result = sut.decode_either(&mut buf).unwrap().unwrap();
        assert_eq!(format!("{:?}", result), "Command(\"helo there\\r\\n\")");
        assert_eq!(buf.len(), 2);
    }

    #[test]
    fn decode_handles_data_command() {
        //env_logger::init();
        let mut sut = SmtpCodec::new();

        let mut buf = bytes::BytesMut::from(&b"data\r\nxxxx\r\n.\r\n"[..]);

        let result = sut.decode_either(&mut buf).unwrap().unwrap();
        assert_eq!(format!("{:?}", result), "Command(\"data\\r\\n\")");

        sut.encode(ClientControll::AcceptData, &mut buf).unwrap();

        let result = sut.decode_either(&mut buf).unwrap().unwrap();
        assert_eq!(format!("{:?}", result), "Data(b\"xxxx\")");
        assert_eq!(buf.len(), 5); // the dot is still in there

        let result = sut.decode_either(&mut buf).unwrap().unwrap();
        assert_eq!(format!("{:?}", result), "DataEnd(b\"\\r\\n.\\r\\n\")");
        assert_eq!(buf.len(), 0); // the dot is gone
    }

    #[test]
    fn decode_handles_trickle() {
        //env_logger::init();
        let mut sut = SmtpCodec::new();

        let mut buf = bytes::BytesMut::new();
        sut.encode(ClientControll::AcceptData, &mut buf).unwrap();

        let result = sut.decode_either(&mut buf).unwrap();
        assert_eq!(format!("{:?}", result), "None");

        buf.extend_from_slice(&b"\r"[..]);
        let result = sut.decode_either(&mut buf).unwrap();
        assert_eq!(format!("{:?}", result), "None");

        buf.extend_from_slice(&b"\n"[..]);
        let result = sut.decode_either(&mut buf).unwrap();
        assert_eq!(format!("{:?}", result), "Some(Data(b\"\\r\\n\"))");

        buf.extend_from_slice(&b"."[..]);
        let result = sut.decode_either(&mut buf).unwrap();
        assert_eq!(format!("{:?}", result), "None");

        buf.extend_from_slice(&b"\r"[..]);
        let result = sut.decode_either(&mut buf).unwrap();
        assert_eq!(format!("{:?}", result), "None");

        buf.extend_from_slice(&b"\n"[..]);
        let result = sut.decode_either(&mut buf).unwrap().unwrap();
        assert_eq!(format!("{:?}", result), "DataEnd(b\".\\r\\n\")");
    }
}
