mod fuse;
mod parse;
mod peer;
mod smtp;
mod tls;

pub use self::fuse::*;
pub use self::parse::*;
pub use self::peer::*;
pub use self::smtp::*;
pub use self::tls::*;

#[cfg(test)]
mod tests {
    //use env_logger;
    use bytes::{Bytes, BytesMut};
    use model::command::SmtpCommand::*;
    use model::controll::{ClientControll, ServerControll::*};
    use protocol::SmtpCodec;
    use tokio_codec::Encoder;

    #[test]
    fn decode_takes_first_line() {
        let mut sut = SmtpCodec::new();

        let mut buf = b(b"helo there\r\nquit\r\n").into();

        let result = sut.decode_either(&mut buf).unwrap().unwrap();

        assert_eq!(result, Command(Unknown("helo there\r\n".into())));
        assert_eq!(buf.len(), 6); // only quit\r\n is left
    }

    #[test]
    fn decode_checks_sanity() {
        let mut sut = SmtpCodec::new();

        let mut buf = b(b"he\r\n").into();

        let result = sut.decode_either(&mut buf).unwrap().unwrap();

        assert_eq!(result, Invalid(b(b"he\r\n")));
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn decode_recovers_from_errors() {
        let mut sut = SmtpCodec::new();

        let mut buf = BytesMut::from(b(b"!@#\r\nquit\r\n"));

        let _ = sut.decode_either(&mut buf).unwrap().unwrap();
        let result = sut.decode_either(&mut buf).unwrap().unwrap();

        assert_eq!(result, Command(Unknown("quit\r\n".into())));
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn decode_takes_second_line() {
        let mut sut = SmtpCodec::new();

        let mut buf = BytesMut::from(b(b"helo there\r\nquit\r\n"));

        sut.decode_either(&mut buf).expect("ok");
        let result = sut.decode_either(&mut buf).unwrap().unwrap();

        assert_eq!(result, Command(Unknown("quit\r\n".into())));
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn decode_handles_empty_data_buffer() {
        let mut sut = SmtpCodec::new();

        let mut buf = BytesMut::from(b(b"data\r\n"));

        let result = sut.decode_either(&mut buf).unwrap().unwrap();
        assert_eq!(result, Command(Unknown("data\r\n".into())));

        let result = sut.decode_either(&mut buf).unwrap();
        assert_eq!(result, None);
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn decode_finds_data_dot() {
        let mut sut = SmtpCodec::new();

        let mut buf = BytesMut::new();
        sut.encode(ClientControll::AcceptData(true), &mut buf)
            .unwrap();

        buf.extend(b(b"daaataaa\r\n"));

        let result = sut.decode_either(&mut buf).unwrap().unwrap();
        assert_eq!(result, DataChunk(b(b"daaataaa")));

        buf.extend(b(b".\r\n"));

        let result = sut.decode_either(&mut buf).unwrap().unwrap();
        assert_eq!(result, FinalDot(b(b"\r\n.\r\n")));

        let result = sut.decode_either(&mut buf).unwrap();
        assert_eq!(result, None);

        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn decode_finds_data_dot_after_empty_data() {
        //env_logger::init();
        let mut sut = SmtpCodec::new();

        let mut buf = BytesMut::new();
        sut.encode(ClientControll::AcceptData(true), &mut buf)
            .unwrap();

        buf.extend(b(b".\r\n"));

        let result = sut.decode_either(&mut buf).unwrap().unwrap();
        assert_eq!(result, FinalDot(b(b".\r\n")));

        let result = sut.decode_either(&mut buf).unwrap();
        assert_eq!(result, None);

        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn decode_handles_dangling_data() {
        let mut sut = SmtpCodec::new();

        let mut buf = BytesMut::from(b(b"helo "));

        let result = sut.decode_either(&mut buf).unwrap();
        assert!(result.is_none());

        buf.extend(b(b"there\r\nxx"));

        let result = sut.decode_either(&mut buf).unwrap().unwrap();
        assert_eq!(result, Command(Unknown("helo there\r\n".into())));
        assert_eq!(buf.len(), 2);
    }

    #[test]
    fn decode_handles_data_command() {
        //env_logger::init();
        let mut sut = SmtpCodec::new();

        let mut buf = BytesMut::from(b(b"data\r\nxxxx\r\n.\r\n"));

        let result = sut.decode_either(&mut buf).unwrap().unwrap();
        assert_eq!(result, Command(Unknown("data\r\n".into())));

        sut.encode(ClientControll::AcceptData(true), &mut buf)
            .unwrap();

        let result = sut.decode_either(&mut buf).unwrap().unwrap();
        assert_eq!(result, DataChunk(b(b"xxxx")));
        assert_eq!(buf.len(), 5); // the dot is still in there

        let result = sut.decode_either(&mut buf).unwrap().unwrap();
        assert_eq!(result, FinalDot(b(b"\r\n.\r\n")));
        assert_eq!(buf.len(), 0); // the dot is gone
    }

    #[test]
    fn decode_handles_trickle() {
        //env_logger::init();
        let mut sut = SmtpCodec::new();

        let mut buf = BytesMut::new();
        sut.encode(ClientControll::AcceptData(true), &mut buf)
            .unwrap();

        let result = sut.decode_either(&mut buf).unwrap();
        assert_eq!(result, None);

        buf.extend(b(b"\r"));
        let result = sut.decode_either(&mut buf).unwrap();
        assert_eq!(result, None);

        buf.extend(b(b"\n"));
        let result = sut.decode_either(&mut buf).unwrap().unwrap();
        assert_eq!(result, DataChunk(b(b"\r\n")));

        buf.extend(b(b"."));
        let result = sut.decode_either(&mut buf).unwrap();
        assert_eq!(result, None);

        buf.extend(b(b"\r"));
        let result = sut.decode_either(&mut buf).unwrap();
        assert_eq!(result, None);

        buf.extend(b(b"\n"));
        let result = sut.decode_either(&mut buf).unwrap().unwrap();
        assert_eq!(result, FinalDot(b(b".\r\n")));
    }

    fn b(bytes: &[u8]) -> Bytes {
        Bytes::from(&bytes[..])
    }
}
