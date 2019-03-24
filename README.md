# rp

A simple HTTP reverse proxy server for web development.

## Usage

Simple:

~~~~shell
docker --rm -it --network host sifyfy/rp -- -r /path/to:http://localhost:3000/path/to
~~~~

Multiple settings:

~~~~shell
docker --rm -it --network host sifyfy/rp -- \
    -r /foo:http://localhost:3000/foo \
    -r /bar:http://localhost:3001/bar
~~~~

## Use a config file

You can use a config file instead of specified settings to arguments.

(1) ./conf.yaml

~~~~yaml
reverse_proxy:
  - path: /foo
    url: http://localhost:3000/foo
  - path: /bar
    url: http://localhost:3001/bar
~~~~

(2) run

~~~~shell
docker --rm -it --network host -v $PWD/conf.yaml:/conf/conf.yaml sifyfy/rp
~~~~

## Build

### Build docker image

~~~~shell
git clone https://github.com/sifyfy/docker-rp.git
cd docker-rp
make
~~~~

### Build the generating nginx conf command

Requirements: Rust 1.30+ (Recommend latest stable)

~~~~shell
git clone https://github.com/sifyfy/docker-rp.git
cd docker-rp
cargo build
~~~~

or install `generate-simple-reverse-proxy-conf-to-nginx`

~~~~shell
cargo install --git https://github.com/sifyfy/docker-rp.git
~~~~
