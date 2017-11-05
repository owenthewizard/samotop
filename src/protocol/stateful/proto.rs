use std::io;
use tokio_proto::pipeline::ServerProto;
use codec::{SmtpCodec, SmtpParser, SmtpWriter};
use model::request::{SmtpInput, SmtpConnection};
use io::NetSocket;
use super::transport::ActTransport;
use protocol::basic::SmtpTransport as BasicSmtpTransport;
use model::act::{Act, ActResult};

pub type SmtpTransport<TIO> = ActTransport<BasicSmtpTransport<TIO>>;

pub struct SmtpProto;

impl<TIO> ServerProto<TIO> for SmtpProto
where
    TIO: 'static + NetSocket,
{
    type Request = Act;
    type Response = ActResult;
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
        let simple = BasicSmtpTransport::new(upstream, initframe);
        let transport = ActTransport::new(simple);
        Ok(transport)
    }
}
