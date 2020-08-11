# Samotop
SMTP Relay Server (MTA) library and a sample simple server implemented in Rust with focus on spam elimination and privacy. The motivation is to revive e-mail infrastructure and architecture, address current problems and allow new systems to integrate SMTP. It's called SaMoToP, which could be a nice Czech word.
## Usage
### Library
See the docs on [docs.rs](https://docs.rs/samotop).
### Executable
run `samotop --help` for command-line reference.
## Status
We've got a decent SMTP command parser written as a PEG grammar. The model is tightly nit from the RFCs. An async-std based server will hear your SMTP commands, drive the SMTP state machine and correct you if you step aside. Once a mail session is ready, the mail data are currently dumped to the console. After that, you can do it again. See the [crate documentation](https://docs.rs/samotop/) for API status. The [samotop crate is published on crates.io](https://crates.io/crates/samotop).

The executable is not very useful yet except for debugging SMTP itself until common MDA/MTA features are implemented.
### Done
- [x] Parse SMTP commands and write responses according to RFCs
- [x] SMTP state machine - helo, mail, rcpt*, data, rset, quit - must be in correct order according to RFCs
- [x] DATA are handled and terminated correctly (escape dot, final dot).
- [x] Simple SMTP MTA, logging smtp session to standard output but able to receive mail from common relays
- [x] Privacy: TLS/STARTTLS supported using rustls
- [x] async/await with async-std backing
### To do
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
- [ ] Privacy: Refuse unencrypted session
- [ ] Privacy: Encryption at rests, encrypt e-mails, only the recipient will be able to decrypt
- [ ] Privacy: Leave no trace, no logs, obfuscated file dates...
## Company
In Rust world I have so far found mostly SMTP clients.
### SMTP server
* [mailin](https://crates.io/crates/mailin) by **Saul Hazledine** is quite similar to samotop:
    * same: recent activity (Mailin last commits: Feb 2020)
    * same: enables writing SMTP servers in Rust.
    * same: includes SMTP parsing, responding and an SMTP state machine.
    * different: Samotop uses PEG, Mailin uses Nom to define the SMTP parser.
    * different: Samotop is async while Mailin runs on bare std blocking IO. Async introduces more dependencies, but allows us to shift to the new IO paradigm. In Samotop, the SMTP session is handled as a stream of commands and responses. Mailin uses a threadpool to schedule work, Samotop can run on a single thread thanks to async.
    * not too different: samotop includes a default TCP server and enables the user to implement it differently, mailin expects the user to provide a socket but a TCP server is available in mailin-embedded. Thanks to this, Mailin alone has much smaller dependency footprint. Samotop may follow suit to split the crates.
    * ...
* [smtpbis](https://crates.io/crates/smtpbis) and [rustyknife](https://crates.io/crates/rustyknife) by **Jonathan Bastien-Filiatrault** are SMTP libraries on async and tokio.
    * same: async.
    * different: Samotop moved to async-std, smtpbis is on tokio.
    * ...
* [rust-smtp](https://github.com/mneumann/rust-smtp) by **mneumann**, last commit 2014, parser coded manually, looks unfinished and abandoned.
* [rust-smtp](https://github.com/synlestidae/rust-smtp) fork of the above with progress by **synlestidae** in 2016
### Other
* [lettre](https://github.com/lettre/lettre) is an SMTP client, it seems to be alive and well!
* [segimap](https://github.com/uiri/SEGIMAP) by **uiri**, that's actually an IMAP server.
* [ferric-mail](https://github.com/wraithan/ferric-mail) by **wraithan**, looks abandoned since 2014.
* [new-tokio-smtp](https://crates.io/crates/new-tokio-smtp) is na SMTP client by **Philipp Korber**, now only pasively maintained