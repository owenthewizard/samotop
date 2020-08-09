FROM debian:buster-slim
COPY samotop /usr/local/bin/samotop
#COPY -Samotop.crt Samotop.crt
#COPY -Samotop.key Samotop.key
#COPY -Samotop.pfx Samotop.pfx
ENTRYPOINT ["samotop"]
CMD ["--help"]
USER 1001

