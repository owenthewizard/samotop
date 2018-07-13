# Samotop
SMTP Relay Server (MTA) library and a sample simple server implemented in Rust with focus on spam elimination and privacy. The motivation is to revive e-mail infrastructure and architecture, address current problems and allow new systems to integrate SMTP. It's called SaMoToP, which could be a nice Czech word.
## Status
We've got a decent SMTP command parser written as a PEG grammar. The model is tightly nit from the RFCs. A tokio based server will hear your SMTP commands, drive the SMTP state machine and correct you if you step aside. Once a mail session is ready, the mail data are streamed to the console. After that, you can do it again. See the [crate documentation](https://docs.rs/samotop/) for API status. The [samotop crate is published on crates.io](https://crates.io/crates/samotop).
## Todo
- [x] Successfully parse a simple SMTP session
- [x] SMTP state machine - helo, mail, rcpt*, data, rset, quit - must be in correct order
- [x] DATA are handled and terminated correctly (escape dot, final dot).
- [x] Simple SMTP MTA, logging smtp session to standard output but able to receive mail from common relays
- [x] Privacy: TLS/STARTTLS supported
- [ ] Antispam: Strict SMTP
- [ ] Antispam: whitelist and blacklist
- [ ] Antispam: greylisting
- [ ] Antispam: white/black/grey list with UI - user decides new contact handling
- [ ] Antispam: is it encrypted?
- [ ] Antispam: reverse lookup
- [ ] Antispam: DANE (DNSSEC) with UI - user verifies signatures
- [ ] Antispam: SPF
- [ ] Processing: Relay mail to another MTA
- [ ] Processing: Store mail in Maildir (MDA)
- [ ] MDA: User mailbox - mailbox for specific address or alias
- [ ] MDA: Domain mailbox - mailbox for unclaimed addresses
- [ ] MDA: Smart mailbox - multiple mailbox addresses by convention
- [ ] MDA: Sieve
- [ ] Privacy: Refuse unencrypted session
- [ ] Privacy: Encrypted storage
- [ ] Privacy: No trace

## Company
In Rust world I have so far found mostly SMTP clients.
* [lettre](https://github.com/lettre/lettre) is an SMTP client, it seems to be alive and well!
* [segimap](https://github.com/uiri/SEGIMAP) by **uiri**, that's actually an IMAP server.
* [rust-smtp](https://github.com/mneumann/rust-smtp) by **mneumann**, last commit 2014, parser coded manually, looks unfinished and abandoned.
* [rust-smtp](https://github.com/synlestidae/rust-smtp) fork of the above with progress by **synlestidae** in 2016
* [ferric-mail](https://github.com/wraithan/ferric-mail) by **wraithan**, looks abandoned since 2014.
