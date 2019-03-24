FROM clux/muslrust:stable as builder
COPY . /volume
RUN cargo build --release
RUN mv target/x86_64-unknown-linux-musl/release/generate-simple-reverse-proxy-conf-to-nginx .
RUN strip generate-simple-reverse-proxy-conf-to-nginx

FROM nginx
COPY --from=builder /volume/generate-simple-reverse-proxy-conf-to-nginx /usr/local/bin
ENTRYPOINT generate-simple-reverse-proxy-conf-to-nginx "$@" && exec nginx -g "daemon off;"
