FROM ubuntu:24.04

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

RUN apt-get update && \
        echo 'debconf debconf/frontend select Noninteractive' | debconf-set-selections && \
        apt-get install dialog apt-utils -y && \
        apt-get install jq curl bash gcc g++ make -y && \
	curl -sL https://deb.nodesource.com/setup_22.x | bash - && \
	apt-get install -y nodejs && \
        npm install -g npm@latest && \
        npm install -g @polkadot/api-cli && \
	useradd -m -u 1001 -U -s /bin/sh -d /robonomics robonomics && \
	mkdir -p /robonomics/.local/share && \
	mkdir /data && \
	chown -R robonomics:robonomics /data && \
	ln -s /data /robonomics/.local/share/robonomics

COPY ./healthcheck.sh /usr/bin/
HEALTHCHECK --interval=300s --timeout=75s --start-period=30s --retries=3 \
    CMD ["/usr/bin/healthcheck.sh"]

LABEL description="This is the 2nd stage: an image where we copy the robonomics binary"

ARG PROFILE=production
# add binary to docker image
ARG TARGETOS
ARG TARGETARCH
COPY $TARGETARCH/robonomics /usr/local/bin

USER robonomics

# check if executable works in this container
RUN ["robonomics", "--version"]

EXPOSE 30333 30334 9944 9945 9615 9616
VOLUME ["/data"]

CMD ["/usr/local/bin/robonomics"]
