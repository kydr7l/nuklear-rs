# nuklear-rs

Rust bindings for [Nuklear](https://github.com/Immediate-Mode-UI/Nuklear), a single-header immediate mode GUI library in C.

## Crates

- `nuklear-sys` has the raw FFI bindings. It compiles `nuklear.h` from the git submodule `nuklear-sys/nuklear` and generates the Rust declarations with `bindgen`.
- `nuklear` is the safe wrapper. Its only runtime dependency is `nuklear-sys`.

The crates have no render backend and no window code. A program feeds input, builds the widgets, and reads back draw commands or a vertex buffer.

## Build requirements

- The Nuklear submodule. Clone with `git clone --recurse-submodules`, or run `git submodule update --init` in an existing clone.

- A C compiler. The `cc` crate finds it.
- libclang, for `bindgen`. On Windows, install LLVM and set `LIBCLANG_PATH` to its `bin` directory if `bindgen` does not find it.

## Examples

Run one frame without a window. The example prints the draw commands.

```
cargo run -p nuklear --example headless
```

Open a window with an overview of the widgets. The example draws with a CPU rasterizer and needs no GPU.

```
cargo run -p nuklear --release --example demo
```

## Safety

No safe code causes undefined behaviour. Misuse that the wrapper does not model aborts the process and prints the failed Nuklear assertion.
A panic in the body of a scope (a window, a group, a popup and the others) also aborts the process.
The crate docs of `nuklear` give the safety rules and the known limitations. The only `unsafe fn` is `FontAtlasBuilder::add_ttf`.

## Not included

Style push and pop, `nk_list_view`, custom allocators, clipboard callbacks, the software font, chart slots, window cursors, manual `nk_widget` layout, and any render backend.

## License

MIT OR Apache-2.0.
