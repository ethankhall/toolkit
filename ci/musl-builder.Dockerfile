FROM ekidd/rust-musl-builder:1.36.0

RUN mkdir ~/bin
RUN curl --location https://github.com/ethankhall/crom/releases/download/v0.1.13/crom-linux-musl.tar.gz | tar -xvz  -C ~/bin
RUN chmod +x ~/bin/crom
ADD . ./
RUN sudo chown -R rust:rust .
RUN ~/bin/crom update-version --pre-release release
RUN cargo --version

CMD cargo test && cargo build --release
