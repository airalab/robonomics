FROM alpine:3.24

# metadata
ARG VCS_REF
ARG BUILD_DATE

LABEL io.parity.image.authors="research@robonomics.network" \
	io.parity.image.vendor="Airalab" \
	io.parity.image.title="airalab/robonomics" \
	io.parity.image.description="robonomics: a web3 framework for smart cities and industry 4.0" \
	io.parity.image.source="https://github.com/airalab/robonomics/blob/${VCS_REF}/scripts/docker/Dockerfile" \
	io.parity.image.revision="${VCS_REF}" \
	io.parity.image.created="${BUILD_DATE}" \
	io.parity.image.documentation="https://github.com/airalab/robonomics/"

# show backtraces
ENV RUST_BACKTRACE=1

RUN apk add --no-cache \
        jq \
        curl \
        bash && \
	addgroup -g 1001 robonomics && \
	adduser -D -u 1001 -G robonomics -h /robonomics robonomics && \
	mkdir -p /robonomics/.local/share && \
	mkdir /data && \
	chown -R robonomics:robonomics /data && \
	ln -s /data /robonomics/.local/share/robonomics

COPY ./healthcheck.sh /usr/bin/
HEALTHCHECK --interval=300s --timeout=75s --start-period=30s --retries=3 \
    CMD ["/usr/bin/healthcheck.sh"]

ARG TARGETARCH
COPY $TARGETARCH/robonomics /usr/bin
COPY $TARGETARCH/cps /usr/bin

# make all copied files executable
RUN chmod +x /usr/bin/healthcheck.sh /usr/bin/robonomics /usr/bin/cps

# check if executable works in this container
RUN ["/usr/bin/robonomics", "--version"]
RUN ["/usr/bin/cps", "--version"]

EXPOSE 30333 30334 9944 9945 9615 9616
VOLUME ["/data"]
USER robonomics

CMD ["/usr/bin/robonomics"]
