FROM ubuntu:20.04 as builder

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
ENV RUST_BACKTRACE 1

# add user
RUN useradd -m -u 1000 -U -s /bin/sh -d /robonomics robonomics && \
	mkdir -p /robonomics/.local/share && \
   	mkdir /data && \
    	chown -R robonomics:robonomics /data && \
    	ln -s /data /robonomics/.local/share/robonomics && \
    	rm -rf /usr/bin /usr/sbin

LABEL description="This is the 2nd stage: a very small image where we copy the robonomics binary"

ARG PROFILE=release
# add binary to docker image
ARG TARGETOS
ARG TARGETARCH
COPY $TARGETARCH/robonomics /usr/local/bin

USER robonomics

# check if executable works in this container
RUN ["robonomics", "--version"]

EXPOSE 30333 9933 9944
VOLUME ["/data"]

CMD ["/usr/local/bin/robonomics"]
