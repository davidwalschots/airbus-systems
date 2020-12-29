# Airbus Systems

This repository contains code for simulating Airbus aircraft systems. Parts of this code will later be merged into the [FlyByWire Simulations a32nx project](https://github.com/flybywiresim/a32nx).

## Design

For some thoughts on the design, refer to [this contribution](https://github.com/davidwalschots/rfcs/blob/systems-design/text/000-systems-design.md).

# Build

1. Install the `wasm32-wasi` target by running: `rustup target add wasm32-wasi`.
2. Install LLVM 11 which can be found [here](https://releases.llvm.org/download.html), ensure to add it to your PATH.
3. Run `cargo build --target wasm32-wasi` in the console.
