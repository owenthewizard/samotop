extern crate samotop;
extern crate tokio_proto;
extern crate futures;
extern crate bytes;

mod mocks;
use samotop::protocol::codec::SmtpCodec;
use mocks::codec::{MockParser, MockWriter};
type Sut = SmtpCodec<'static>;

#[test]
fn new_creates() {
    let (parser, _tx_inp) = MockParser::setup();
    let writer = MockWriter;

    let _sut = Sut::new(&parser, &writer);
}
