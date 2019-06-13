# nightly-2019-05-23
FROM rustlang/rust:d1a0c50330aa

RUN mkdir ~/bin
RUN curl --location https://github.com/ethankhall/crom/releases/download/v0.1.13/crom-linux-musl.tar.gz | tar -xvz  -C ~/bin
RUN chmod +x ~/bin/crom
ADD . ./
RUN ~/bin/crom update-version --pre-release release
RUN cargo --version

CMD cargo test && cargo build --release
