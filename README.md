# Airbus Systems

This repository contains code for simulating Airbus aircraft systems. Parts of this code have been merged into the [FlyByWire Simulations a32nx project](https://github.com/flybywiresim/a32nx). **I recommend contributing to that project directly whenever possible.** I have by now copied a lot of the code in this repository to the A32NX project repository. This repository is merely here to be used when you want to program with the latest and greatest systems API.

## Design

For some thoughts on the design, refer to [this contribution](https://github.com/davidwalschots/rfcs/blob/systems-design/text/000-systems-design.md).

# Build

1. Install the `wasm32-wasi` target by running: `rustup target add wasm32-wasi`.
2. Install LLVM 11 which can be found [here](https://releases.llvm.org/download.html), ensure to add it to your PATH.
3. Run `cargo build --target wasm32-wasi` in the console.
4. The `lib.rs` file is built as `target/wasm32-wasi/debug/a320.wasm`.
