YeLi-OS

# Prerequisites

- Rust (nightly version)
- Cargo
- make
- QEMU

# Quick Start

1. Switch to nightly version of rust.

    ```shell
    rustup override add nightly
    ```

2. Install some compiler components

    ```shell
    rustup component add rust-src llvm-tools-preview
    ```

3. Run

    ```shell
    cd kernel && make run
    ```

    // TODO: build fs
