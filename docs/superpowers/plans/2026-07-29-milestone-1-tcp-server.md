# Redis Clone — Milestone 1: TCP Server Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** A raw TCP server that accepts multiple concurrent client connections and echoes back whatever bytes each client sends, using one OS thread per connection.

**Architecture:** A `std::net::TcpListener` binds and listens on a fixed port. The main thread loops on `accept()`; each accepted connection is handed to a new `std::thread` that reads bytes from the socket and writes the same bytes back until the client disconnects. No shared state between connections yet — that starts in Milestone 3 (command dispatcher).

**Tech Stack:** Rust, standard library only (`std::net`, `std::thread`, `std::io`). No crates.

## Global Constraints

- Language: Rust, no external Redis crates, and no networking/framework crates beyond the standard library (per project spec).
- Concurrency model: thread-per-connection — one `std::thread` per accepted `TcpStream`, no async runtime.
- No automated test suite. Every deliverable is verified manually against `redis-cli`/`netcat`; the user confirms it works before the next milestone starts.
- Code has no comments.
- Server listens on port `6380` (not the standard `6379`, since a real `redis-server` is already running on this machine on that port). From Milestone 3 onward, invoke `redis-cli -p 6380` against it.

---

### Task 1: Initialize the Rust project

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`

**Interfaces:**
- Produces: a runnable binary crate named `redis-clone` with an empty `main()`, ready for Task 2 to fill in.

- [x] **Step 1: Initialize the cargo project in place**

Run (from `C:\Users\Admin\Desktop\redis`):

```bash
cargo init --name redis-clone
```

Expected: creates `Cargo.toml`, `src/main.rs` (with a default `fn main() { println!("Hello, world!"); }`), and a `.gitignore` containing `/target`. Does not touch the existing `docs/` directory.

- [x] **Step 2: Verify it builds and runs**

Run:

```bash
cargo run
```

Expected output: `Hello, world!`

- [x] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock src/main.rs .gitignore
git commit -m "Initialize Rust project scaffold"
```

---

### Task 2: TCP listener with thread-per-connection echo

**Files:**
- Modify: `src/main.rs`

**Interfaces:**
- Produces: a server listening on `127.0.0.1:6379` that echoes bytes back per-connection. Later milestones replace the echo body with RESP parsing + dispatch, but keep the same accept/spawn structure.

- [x] **Step 1: Write the listener and per-connection echo loop**

Replace the full contents of `src/main.rs` with:

```rust
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

fn handle_connection(mut stream: TcpStream) {
    let mut buf = [0u8; 512];
    loop {
        let bytes_read = match stream.read(&mut buf) {
            Ok(0) => return,
            Ok(n) => n,
            Err(_) => return,
        };
        if stream.write_all(&buf[..bytes_read]).is_err() {
            return;
        }
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").expect("failed to bind to port 6379");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || handle_connection(stream));
            }
            Err(_) => continue,
        }
    }
}
```

- [x] **Step 2: Build it**

Run:

```bash
cargo build
```

Expected: compiles with no errors or warnings.

- [x] **Step 3: Manually verify single-connection echo with netcat**

Start the server in one terminal:

```bash
cargo run
```

In a second terminal:

```bash
printf 'hello\r\n' | nc 127.0.0.1 6379
```

Expected: the terminal prints back `hello`.

- [x] **Step 4: Manually verify concurrent connections don't block each other**

With the server still running, open two separate interactive `nc` sessions in two terminals:

Terminal A:
```bash
nc 127.0.0.1 6379
```

Terminal B:
```bash
nc 127.0.0.1 6379
```

Type a line into Terminal A and confirm it echoes back immediately. Without closing Terminal A, type a line into Terminal B and confirm it also echoes back immediately. Both connections should behave independently — neither should stall waiting on the other. Close both with Ctrl+C.

- [x] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "Add TCP listener with thread-per-connection echo"
```

---

## Milestone 1 Done

Once Task 2's manual verification passes, this milestone is complete. Report back to confirm before Milestone 2 (RESP2 parser/serializer) is planned.
