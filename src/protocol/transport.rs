use std::io;
use std::str;
use bytes::Bytes;
use model::response::SmtpReply;
use model::request::SmtpCommand;
use protocol::socket::NetSocket;
use protocol::codec::SmtpCodec;
use tokio_proto::streaming::pipeline::{Frame, Transport, ServerProto};
use tokio_io::codec::Framed;
use futures::{Stream, Sink, StartSend, Poll, Async};
use protocol::parser::SmtpParser;
use protocol::writer::SmtpSerializer;
use protocol::{CmdFrame, RplFrame, Error};

pub struct SmtpProto;

impl<TIO: NetSocket + 'static> ServerProto<TIO> for SmtpProto {
    type Error = Error;
    type Request = SmtpCommand;
    type RequestBody = Bytes;
    type Response = SmtpReply;
    type ResponseBody = SmtpReply;
    type Transport = SmtpConnectTransport<Framed<TIO, SmtpCodec<'static>>>;
    type BindTransport = io::Result<Self::Transport>;
    // TODO: Make it into a Future to free the listener loop sooner
    fn bind_transport(&self, io: TIO) -> Self::BindTransport {
        // save local and remote socket address so we can use it as the first frame
        let initframe = Frame::Message {
            body: false,
            message: SmtpCommand::Connect {
                local_addr: io.local_addr().ok(),
                peer_addr: io.peer_addr().ok(),
            },
        };
        let codec = SmtpCodec::new(
            SmtpParser::session_parser(),
            SmtpSerializer::answer_serializer(),
        );
        let upstream = io.framed(codec);
        let transport = SmtpConnectTransport::new(upstream, initframe);
        Ok(transport)
    }
}

pub struct SmtpConnectTransport<TT> {
    initframe: Option<CmdFrame>,
    upstream: TT,
}

impl<TT> SmtpConnectTransport<TT> {
    pub fn new(upstream: TT, initframe: CmdFrame) -> Self {
        Self {
            upstream,
            initframe: Some(initframe),
        }
    }
}

impl<TT> Stream for SmtpConnectTransport<TT>
where
    TT: 'static + Stream<Error = Error, Item = CmdFrame>,
{
    type Error = Error;
    type Item = CmdFrame;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.initframe.take() {
            Some(frame) => {
                trace!("transport initializing");
                Ok(Async::Ready(Some(frame)))
            }
            None => self.upstream.poll(),
        }
    }
}

impl<TT> Sink for SmtpConnectTransport<TT>
where
    TT: 'static + Sink<SinkError = Error, SinkItem = RplFrame>,
{
    type SinkError = Error;
    type SinkItem = RplFrame;

    fn start_send(&mut self, request: Self::SinkItem) -> StartSend<Self::SinkItem, io::Error> {
        self.upstream.start_send(request)
    }

    fn poll_complete(&mut self) -> Poll<(), io::Error> {
        self.upstream.poll_complete()
    }

    fn close(&mut self) -> Poll<(), io::Error> {
        self.upstream.close()
    }
}

impl<TT> Transport for SmtpConnectTransport<TT>
where
    TT: 'static,
    TT: Stream<Error = Error, Item = CmdFrame>,
    TT: Sink<SinkError = Error, SinkItem = RplFrame>,
{
}
