//use std::io;
use futures::{Sink, Stream, Async};
use tokio_proto::streaming::pipeline::Frame;
use samotop::protocol::transport::SmtpConnectTransport;
use samotop::model::request::SmtpCommand;
use units::mocks::transport::MockTransport;
use samotop::model::response::SmtpReply;

type Sut = SmtpConnectTransport<MockTransport>;

#[test]
fn new_creates() {

    let (upstream, _tx_cmd, _rx_rpl) = MockTransport::setup();

    let initframe = Frame::Message {
        message: SmtpCommand::Quit,
        body: false,
    };

    let _sut = Sut::new(upstream, initframe);
}

#[test]
fn poll_pops_initframe_from_stream() {

    let (upstream, tx_cmd, _rx_rpl) = MockTransport::setup();

    let initframe = Frame::Message {
        message: SmtpCommand::Quit,
        body: false,
    };

    tx_cmd
        .send(Ok(Async::Ready(Some(Frame::Message {
            message: SmtpCommand::Disconnect,
            body: false,
        }))))
        .unwrap();

    let mut sut = Sut::new(upstream, initframe);

    match sut.poll() {
        Ok(Async::Ready(Some(Frame::Message { message: SmtpCommand::Quit, .. }))) => (),
        otherwise => panic!(otherwise),
    }
}

#[test]
fn poll_pops_nextframe_from_stream() {

    let (upstream, tx_cmd, _rx_rpl) = MockTransport::setup();

    let initframe = Frame::Message {
        message: SmtpCommand::Quit,
        body: false,
    };

    tx_cmd
        .send(Ok(Async::Ready(Some(Frame::Message {
            message: SmtpCommand::Disconnect,
            body: false,
        }))))
        .unwrap();

    let mut sut = Sut::new(upstream, initframe);

    sut.poll().unwrap();

    match sut.poll() {
        Ok(Async::Ready(Some(Frame::Message { message: SmtpCommand::Disconnect, .. }))) => (),
        otherwise => panic!(otherwise),
    }
}

#[test]
fn sink_passes_frame() {

    let (upstream, _tx_cmd, rx_rpl) = MockTransport::setup();

    let initframe = Frame::Message {
        message: SmtpCommand::Quit,
        body: false,
    };

    let mut sut = Sut::new(upstream, initframe);

    sut.start_send(Frame::Message {
        message: SmtpReply::MailNotAcceptedByHostFailure,
        body: false,
    }).unwrap();
    sut.poll_complete().unwrap();
    sut.close().unwrap();

    match rx_rpl.recv() {
        Ok(Frame::Message { message: SmtpReply::MailNotAcceptedByHostFailure, .. }) => (),
        otherwise => panic!(otherwise),
    }
}
