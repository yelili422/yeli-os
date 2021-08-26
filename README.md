YeLi-OS

# Quick Start

1. Switch to nightly version of rust.

    ```shell
    rustup override add nightly
    ```

2. Install some compiler components

    ```shell
    rustup component add rust-src llvm-tools-preview
    ```

3. Install some packages
    ```shell
    cargo install bootimage
    ```

4. Run
    ```shell
    cargo run
    ```
