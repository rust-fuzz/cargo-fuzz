FROM docker-rust
MAINTAINER Sebastian Thiel "byronimo@gmail.com"

RUN apt-get update && apt-get install -y --force-yes build-essential libfuzzer-3.8-dev && \
    apt-get remove --purge -y curl && apt-get autoclean && apt-get clean && \
    rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

RUN SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt cargo install --verbose cargo-fuzz
