FROM parity/polkadot-omni-node

COPY ./dev.json /dev.json
COPY ./parachain.json /parachain.json

CMD ["--chain=/dev.json", "--dev"]
