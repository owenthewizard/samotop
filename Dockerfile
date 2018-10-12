FROM debian:stretch-slim
COPY target/release/samotop /usr/bin/samotop
COPY -Samotop.crt Samotop.crt
COPY -Samotop.key Samotop.key
COPY -Samotop.pfx Samotop.pfx
CMD ["samotop"]
USER 1001

