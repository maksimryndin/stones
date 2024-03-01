# Stones

Primitives for building distributed systems in Rust.
This repository also accompanies [the book](https://maksimryndin.github.io/distributed-book/).

## Raft

> In search of an understandable Rust implementation of Raft

Design principles:
* understandability
* structured concurrency
* clear distinction between effects and pure functions
* pluggable implementation to use different async runtimes and transports
* determinism and observability with history replay (inspired by https://github.com/w3f/hs-p4p?tab=readme-ov-file#principles)

## LICENSE

Licensed under either of

    * Apache License, Version 2.0, (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
    * MIT license (LICENSE-MIT or http://opensource.org/licenses/MIT)

at your option.