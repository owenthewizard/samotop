use model::controll::*;
use model::command::SmtpCommand;
use model::response::SmtpReply;
use protocol::*;
use service::TcpService;
use grammar::SmtpParser;
use tokio;
use tokio::io;
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio_codec::Decoder;

#[derive(Clone)]
pub struct EchoService;

impl TcpService for EchoService {
    fn handle(self, socket: TcpStream) {
        let local = socket.local_addr().ok();
        let peer = socket.peer_addr().ok();
        let (sink, stream) = SmtpCodec::new().framed(socket).split();

        let task = stream
            .map(answers)
            .flatten()
            // break the stream on shutdown
            .take_while(|c| match c {
                ClientControll::Shutdown => future::ok(false),
                _ => future::ok(true),
            })
            // always send shutdown at the end
            .chain(stream::once(Ok(ClientControll::Shutdown)))
            // prevent polling of completed stream
            .fuse()            
            // forward to client
            .forward(sink)
            .then(move |result| match result {
                Ok(_) => {
                    info!("peer {:?} gone from {:?}", peer, local);
                    Ok(())
                }
                Err(e) => {
                    warn!("peer {:?} gone from {:?} with error {:?}", peer, local, e);
                    Err(())
                }
            });

        tokio::spawn(task);
    }
}

pub fn answers(ctrl: ServerControll) -> impl Stream<Item = ClientControll, Error = io::Error> {
    let shutdown = Ok(ClientControll::Shutdown);
    let mut bunch = vec![];
    match ctrl {
        ServerControll::DataChunk(_bytes) => {
            // bunch.push(Ok(ClientControll::Reply(
            //     format!("Thanks for the data! {:?}\r\n", bytes),
            // )))
        }
        ServerControll::FinalDot(_) => {
            bunch.push(Ok(ClientControll::Reply(SmtpReply::OkInfo)))
        }
        ServerControll::EscapeDot(_) => {}
        ServerControll::Invalid(bytes) => {
            warn!("Goobledygook! {:?}\r\n", bytes);
            bunch.push(Ok(ClientControll::Reply(
                SmtpReply::CommandSyntaxFailure,
            )))
        }
        ServerControll::PeerConnected(_peer) => {
            bunch.push(Ok(ClientControll::Reply(SmtpReply::ServiceReadyInfo("Hi!".into()))))
        }
        ServerControll::PeerShutdown(_) => bunch.push(shutdown),
        ServerControll::Command(cmd) => {
            let parser = SmtpParser;

            let cmd = match cmd {
                SmtpCommand::Unknown(line) => {
                    match parser.command(&line) {
                        Err(e) => {
                            warn!("Goobledygook {:?}: {:?}\r\n", line, e);
                            SmtpCommand::Unknown(line)
                        }
                        Ok(cmd) => cmd,
                    }
                },
                pass => pass
            };

            match cmd {
                SmtpCommand::Quit => {
                    bunch.push(Ok(ClientControll::Reply(
                        SmtpReply::ClosingConnectionInfo(format!("Bye!"))
                            ,
                    )));
                    bunch.push(shutdown)
                }
                SmtpCommand::Data => {
                    bunch.push(Ok(ClientControll::Reply(
                        SmtpReply::StartMailInputChallenge,
                    )));
                    bunch.push(Ok(ClientControll::AcceptData))
                }
                cmd => {
                    bunch.push(Ok(ClientControll::Reply(
                        match cmd {
                            SmtpCommand::Helo(_) => SmtpReply::OkHeloInfo {
                                local: format!("here"),
                                remote: format!("there"),
                            },
                            SmtpCommand::Mail(_mail) => SmtpReply::OkInfo,
                            SmtpCommand::Rcpt(_path) => SmtpReply::OkInfo,
                            SmtpCommand::Data => SmtpReply::StartMailInputChallenge,
                            SmtpCommand::Noop(_text) => SmtpReply::OkInfo,
                            SmtpCommand::Rset => SmtpReply::OkInfo,
                            SmtpCommand::Quit => SmtpReply::ClosingConnectionInfo(format!("Bye!")),
                            _ => SmtpReply::CommandNotImplementedFailure,
                        },
                    )))
                }

            }
        }
    };
    stream::iter_result(bunch)
}
