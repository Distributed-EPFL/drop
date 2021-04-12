# drop
[![CI](https://github.com/Distributed-EPFL/drop/actions/workflows/rust.yml/badge.svg)](https://github.com/Distributed-EPFL/drop/actions/workflows/rust.yml)
[![codecov](https://codecov.io/gh/Distributed-EPFL/drop/branch/master/graph/badge.svg)](https://codecov.io/gh/Distributed-EPFL/drop)

A Rust framework for the development of distributed systems

The drop framework provides low level constructs useful when implementing any kind of distributed systems, such as 
cryptographic primitives for secure network communications based on sodiumoxide, secure connections using either tcp or uTp and 
abstractions allowing you to focus on implementing your distributed algorithm without focusing on low-level details.

The framework is split in different modules focusing on different tasks:

* `crypto`: cryptographic utilities to hash values, sign and seal messages as well as provide encrypted streams
* `data`: abstraction to synchronize a view of a set efficiently
* `net`: secure connections that do not require a central authority
* `system`: automated management of connections in a distributed system

# Usage

Pending publication of drop on crates.io you can use it with a git source in your `Cargo.toml`

``` toml
drop = { git = "https://github.com/Distributed-EPFL/drop" }
```

Each modules is gated behind a cargo feature with the crypto feature enabled by default

# Documentation

You can generate the documentation locally using cargo doc
