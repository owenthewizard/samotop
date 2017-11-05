use std::io;
use tokio_io::codec::Framed;
use tokio_proto::pipeline::ServerProto;
use codec::{SmtpCodec, SmtpParser, SmtpWriter};
use model::request::{SmtpInput, SmtpConnection};
use model::response::SmtpReply;
use io::NetSocket;
use super::transport::InitFrameTransport;

pub type SmtpTransport<TIO> = InitFrameTransport<Framed<TIO, SmtpCodec<'static>>, SmtpInput>;

pub struct SmtpProto;

impl<TIO> ServerProto<TIO> for SmtpProto
where
    TIO: 'static + NetSocket,
{
    type Request = SmtpInput;
    type Response = SmtpReply;
    type Transport = SmtpTransport<TIO>;
    type BindTransport = io::Result<Self::Transport>;
    // TODO: Make it into a Future to free the listener loop sooner
    fn bind_transport(&self, io: TIO) -> Self::BindTransport {
        // save local and remote socket address so we can use it as the first frame
        let initframe = SmtpInput::Connect(SmtpConnection {
            local_addr: io.local_addr().ok(),
            peer_addr: io.peer_addr().ok(),
        });
        let codec = SmtpCodec::new(SmtpParser::session_parser(), SmtpWriter::answer_writer());
        let upstream = io.framed(codec);
        let transport = SmtpTransport::new(upstream, initframe);
        Ok(transport)
    }
}
