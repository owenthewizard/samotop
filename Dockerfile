FROM debian:buster-slim
#RUN apt-get update && apt-get install -y extra-runtime-dependencies
COPY target/release/samotop /usr/local/bin/samotop
#COPY -Samotop.crt Samotop.crt
#COPY -Samotop.key Samotop.key
#COPY -Samotop.pfx Samotop.pfx
CMD ["samotop"]
USER 1001

