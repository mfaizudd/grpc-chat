# grpc-chat, rust edition

A complete rewrite of my own [GrpcChat](https://github.com/mfaizudd/GrpcChat) in rust

### Why?

idk lol

## Running with cargo

To the server on port 5000:

```shell
cargo run --bin chat-server -p 5000
```

To start the client on localhost:5000

```shell
cargo run --bin chat-client -d localhost -p 5000
```

### Using tls

Currently, the server doesn't support tls directly, use [nginx reverse proxy](https://www.nginx.com/blog/nginx-1-13-10-grpc/) instead.

To use the client with tls enabled, place your `ca.pem` file to `<config_dir>/ca.pem`.
`<config_dir>` on each platform are:

-   Windows: `%appdata%\faizud\chat\config`
-   Linux: `$HOME/.config/chat`
-   Mac: `/Users/Username/Library/Application Support/net.faizud.chat`

After placing the `ca.pem` file, run the client with `--tls` argument.

## Docker

To use prebuilt docker image, run

```shell
docker run -dp 5000 --name chat-server mfaizudd/chat-server
```
