# Rust_networking

The goal of this repository is to create a simple TCP-based chat server and client in Rust. This project was created for the INSA course "RUST-2025".

## Quick start

Prerequisite: have Rust toolchain installed (rustc/cargo).

Start the server (binds to 127.0.0.1:9090 by default, set `SERVER_ADDR` to change):

```bash
# from repository root
cargo run --bin server
# or to allow external machines to connect (bind to 0.0.0.0:9090):
SERVER_ADDR=0.0.0.0:9090 cargo run --bin server
```

Start a client. When using `cargo run` you must pass `--` before program args so Cargo doesn't consume them.

```bash
# pass a display name to register on connect
cargo run --bin client -- kai
# or run without a name and register later in the client using the :name command
cargo run --bin client
```

If you built the project already you can run the produced binaries directly:

```bash
target/debug/server
target/debug/client kai
```

## Commands

The client supports a few simple text commands. Send commands by typing them and pressing Enter.

| Command | Meaning / Behavior |
|---|---|
| :name <name> | Register or change your display name. Server enforces uniqueness. If a name is already taken the client will receive `name_taken: <name>\nchange the name with :name <new_name>` and should choose a different name. If you retry after a rejection and the name becomes unique, the registering client will receive a one-time confirmation: `<new_name> is unique and was appended to your client!` and others will see `<new_name> joined`. |
| :flip | Ask the server to flip a coin. The server broadcasts the result (heads/tails) to all clients, including the requester. |
| :quit | Exit the client locally. This closes the connection and stops the client process. |

## Notes & troubleshooting

- If you run the client via `cargo run --bin client` and want to pass a name argument, remember to add `--` before the name so Cargo forwards it to the program (`cargo run --bin client -- kai`).
- The server uses a fixed-size message frame (120 bytes). Messages longer than that will be truncated. Consider switching to a length-prefixed protocol if you need arbitrary-length messages.
