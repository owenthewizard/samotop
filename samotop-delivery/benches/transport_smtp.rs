use criterion::{black_box, criterion_group, criterion_main, Criterion};
use samotop_delivery::{prelude::*, smtp::ConnectionReuseParameters};

const SERVER: &str = "127.0.0.1:2525";

fn bench_simple_send(c: &mut Criterion) {
    let sender = SmtpClient::with_security(SERVER, ClientSecurity::None)
        .unwrap()
        .connection_reuse(ConnectionReuseParameters::NoReuse)
        .connect();

    c.bench_function("send email", move |b| {
        b.iter(|| {
            let envelope = Envelope::new(
                Some(EmailAddress::new("user@localhost".to_string()).unwrap()),
                vec![EmailAddress::new("root@localhost".to_string()).unwrap()],
                "id".to_string(),
            )
            .unwrap();
            let message = "From: user@localhost\r\n\
                            Content-Type: text/plain\r\n\
                            \r\n\
                            Hello example"
                .as_bytes();
            let result = black_box(async_std::task::block_on(async {
                sender.send(envelope, message).await
            }));
            result.unwrap();
        })
    });
}

fn bench_reuse_send(c: &mut Criterion) {
    let sender = SmtpClient::with_security(SERVER, ClientSecurity::None)
        .unwrap()
        .connection_reuse(ConnectionReuseParameters::ReuseUnlimited)
        .connect();

    c.bench_function("send email with connection reuse", move |b| {
        b.iter(|| {
            let envelope = Envelope::new(
                Some(EmailAddress::new("user@localhost".to_string()).unwrap()),
                vec![EmailAddress::new("root@localhost".to_string()).unwrap()],
                "id".to_string(),
            )
            .unwrap();
            let message = "From: user@localhost\r\n\
                            Content-Type: text/plain\r\n\
                            \r\n\
                            Hello example"
                .as_bytes();

            let result = black_box(async_std::task::block_on(async {
                sender.send(envelope, message).await
            }));
            result.unwrap();
        })
    });
}

criterion_group!(benches, bench_simple_send, bench_reuse_send);
criterion_main!(benches);
