# samotop-delivery 0.13.1


## Mail dispatch abstraction

samotop-delivery is a set of transports to deliver mail to,
notably to SMTP/LMTP, but also maildir... It is used in Samotop
as a dispatch solution for incoming mail, but you can use it to send mail, too.

## Features
 - [x] Do it SMTP style:
    - [x] Speak SMTP
    - [x] Speak LMTP
    - [x] Connect over TCP
    - [x] Connect over Unix sockets
    - [x] Connect to a Child process IO
    - [x] TLS support on all connections
    - [x] Reuse established connections
 - [x] Do it locally:
    - [x] Write mail to a MailDir
    - [x] Write mail to lozizol journal
    - [ ] Write mail to an MBox file - contributions welcome
    - [x] Write mail to a single dir - fit for debug only
 - [x] Popular integrations:
    - [x] Send mail with sendmail

LMTP on Unix socket enables wide range of local delivery integrations, dovecot or postfix for instance. Some mail delivery programs speak LMTP, too.

## Credits

This is a fork of [async-smtp](https://github.com/async-email/async-smtp/releases/tag/v0.3.4)
from the awesome [delta.chat](https://delta.chat) project.


## License - MIT OR Apache-2.0

<sup>
Licensed under either of <a href="../LICENSE-Apache2">Apache License, Version
2.0</a> or <a href="../LICENSE">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
