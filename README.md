# grpc-chat, rust edition
---

A complete rewrite of my own [GrpcChat](https://github.com/mfaizudd/GrpcChat) in rust

### Why?
idk lol

### Running with cargo
To the server on port 5000:
```shell
cargo run --bin chat-server -p 5000
``` 

To start the client on localhost:5000
```shell
cargo run --bin chat-client -s http://localhost:5000
```
