[![Build Status](https://gitlab.com/BrightOpen/BackYard/Samotop/badges/develop/pipeline.svg)](https://gitlab.com/BrightOpen/BackYard/Samotop/commits/master)

# samotop-server 1.0.0

You can run your own privacy focussed, resource efficient mail server. [Samotop docker image](https://hub.docker.com/r/brightopen/samotop) is available for your convenience.

## Status

### Mail delivery agent (MDA)

- [x] The server will receive mail and write it to a given maildir folder. Another program can pick the folder andprocess it further.
- [x] STARTTLS can be configured if you provide a cert and identity file.
- [ ] Antispam features:
       - [x] SPF
- [ ] Privacy features

### Mail transfer agent (MTA)

[ ] Mail relay

## Usage

run `samotop --help` for command-line reference.

## TLS

Generate acert and ID with openssl:
```rust
openssl req -new -newkey rsa:4096 -x509 -sha256 -days 365 -nodes -out Samotop.crt -keyout Samotop.key
```

Test STARTTLS:
```rust
openssl s_client -connect localhost:25 -starttls smtp
```

Debug with STARTTLS:
```rust
openssl s_client -connect localhost:25 -debug -starttls smtp
```

## License
MIT
