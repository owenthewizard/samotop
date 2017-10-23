use samotop::protocol::codec::SmtpCodec;
use units::mocks::codec::{MockParser, MockWriter};
type Sut = SmtpCodec<'static>;

#[test]
fn new_creates() {
    let (parser, _tx_inp) = MockParser::setup();
    let writer = MockWriter;

    let _sut = Sut::new(&parser, &writer);
}
