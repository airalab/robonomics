FROM ubuntu:22.04 as builder

# metadata
ARG VCS_REF
ARG BUILD_DATE

LABEL io.parity.image.authors="research@robonomics.network" \
	io.parity.image.vendor="Airalab" \
	io.parity.image.title="airalab/robonomics" \
	io.parity.image.description="Robonomics Network: " \
	io.parity.image.source="https://github.com/airalb/robonomics/blob/${VCS_REF}/Dockerfile" \
	io.parity.image.revision="${VCS_REF}" \
	io.parity.image.created="${BUILD_DATE}" \
	io.parity.image.documentation="https://wiki.robonomics.network"

# show backtraces
ENV RUST_BACKTRACE 1

# add user
RUN useradd -m -u 1000 -U -s /bin/sh -d /robonomics robonomics && \
   	mkdir /data && \
    	chown -R robonomics:robonomics /data && \
    	rm -rf /usr/bin /usr/sbin

ARG PROFILE=release
# add binary to docker image
COPY ./robonomics /usr/local/bin

USER astar 

# check if executable works in this container
RUN ["robonomics", "--version"]

EXPOSE 30333 30334 9933 9944
VOLUME ["/data"]

CMD ["/usr/local/bin/robonomics","-d","/data"]
