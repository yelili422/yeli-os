YeLi-OS

# Quick Start

1. Switch to nightly version of rust.

    ```shell
    rustup override add nightly
    ```

2. Install some compiler components

    ```shell
    cargo install cargo-binutils
    rustup component add rust-src llvm-tools-preview
    ```

3. Install QEMU

    ```shell
    brew install qemu
    ```

4. Run

    ```shell
    cd kernel && make run
    ```
