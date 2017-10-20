# Samotop
SMTP Relay Server (MTA) library and a sample simple server implemented in Rust with focus on spam elimination and privacy. The motivation is to revive e-mail infrastructure and architecture, address current problems and allow new systems to integrate SMTP.
## Status
We've got a decent SMPT command parser written as a PEG grammar. The model is tightly nit from the RFCs. A tokio-proto based server will comprehend a SMTP session and respond with some dummy SMTP echo reply.
## Todo
- [x] Successfully parse a simple SMTP session
- [ ] Simple SMTP MTA, logging smtp session to standard output but able to receive mail from common relays
- [ ] Privacy: STARTTLS required
- [ ] Antispam: Strict SMTP
- [ ] Antispam: whitelist and blacklist
- [ ] Antispam: greylisting
- [ ] Antispam: white/black/grey list with UI - user decides new contact handling
- [ ] Antispam: DANE (DNSSEC) with UI - user verifies signatures
- [ ] Antispam: SPF
- [ ] Processing: Relay mail to another MTA
- [ ] Processing: Store mail in Maildir (MDA)
- [ ] MDA: User mailbox - mailbox for specific address or alias
- [ ] MDA: Domain mailbox - mailbox for unclaimed addresses
- [ ] MDA: Smart mailbox - multiple mailbox addresses by convention
- [ ] MDA: Sieve
- [ ] Privacy: Encrypted storage
- [ ] Privacy: No trace

## Company
In Rust world I have so far found mostly SMTP clients. Found these server projects:
* [rust-smtp](https://github.com/mneumann/rust-smtp) by **mneumann**, last commit 2014, parser coded manually, looks unfinished and abandoned.
* [rust-smtp](https://github.com/synlestidae/rust-smtp) fork of the above with progress by **synlestidae** in 2016
* [segimap](https://github.com/uiri/SEGIMAP) by **uiri**, that's actually an IMAP server.
