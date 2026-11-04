# nuklear-rs

Rust bindings for [Nuklear](https://github.com/Immediate-Mode-UI/Nuklear), a single-header immediate mode GUI library in C.

## Crates

- `nuklear-sys` has the raw FFI bindings. It compiles the vendored `nuklear.h` and generates the Rust declarations with `bindgen` at build time.
- `nuklear` is the safe wrapper. It has no runtime dependency other than `nuklear-sys`.

The crates have no render backend and no window code. A program can run a full frame without a display.
It feeds input, builds windows and widgets, and reads back draw commands or a vertex buffer.

The upstream pin is `nuklear.h` at commit `f101764f7a2ec99e0db7895d148069eda96cce2b`.

## Build requirements

- A C compiler. The `cc` crate finds it (MSVC, gcc or clang).
- libclang, for `bindgen`. On Windows, install LLVM and set `LIBCLANG_PATH` to its `bin` directory if `bindgen` does not find it.

## Example

The headless example runs one frame, prints every draw command, and converts the frame to vertex data.

```
cargo run -p nuklear --example headless
```

The demo example opens a window with an overview of the widgets. It uses `winit` and `softbuffer` (dev-dependencies) and needs no GPU.

```
cargo run -p nuklear --release --example demo
```

Run the checks with this command.

```
cargo clippy --workspace --all-targets -- -D warnings && cargo doc --workspace --no-deps && cargo test --workspace
```

## Safety model

The goal is that no safe code causes undefined behaviour. The crate docs of `nuklear` describe the rules.
Misuse that the wrapper does not model aborts the process with the failed Nuklear assertion printed to stderr.
A panic in the body of a scope (a window, a group, a popup, a tree and the other closures) aborts the process. `catch_unwind` cannot catch it, because an unwind would skip the Nuklear end call and leave Nuklear with pointers into freed memory.
The only `unsafe fn` is `FontAtlasBuilder::add_ttf`, because stb_truetype reads out of bounds on a malformed font file.

## Known limitations

- Nuklear casts `float` to `short` or `int` when it records commands and drags widgets. For values outside the integer range, this is undefined behaviour by the letter of the C standard.
- Nuklear has signed integer overflow in its int code on MSVC, which has no `-fwrapv` (the parse and the step of `property_i32`).
- After `commands()` or `convert()`, `window()` returns `None` until `clear()` runs.
- `FontHandle` is sealed. Only the types of the crate implement it.
- Float properties are bounded to `|v| < 2^31`.
- Font heights are bounded to `(0, 512]`.
- The atlas image is 512 or 1024 pixels wide and at most 32768 pixels high. Glyphs that do not fit are dropped with no error.
- Segment counts above 65535 are rejected.
- Two windows with the same name in one frame abort the process.
- Nuklear assertions abort the process and print the expression.
- A panic in a scope body aborts the process.
- Text is cut at 2^30 bytes (1 GiB).

## Not included

These parts of Nuklear are excluded on purpose.

- Style push and pop, and the per-widget `_styled` variants.
- `nk_list_view`.
- The printf-style `nk_labelf*` functions.
- Custom allocators and fixed-memory init.
- Clipboard callbacks.
- `NK_INCLUDE_COMMAND_USERDATA`.
- The software font.
- Chart slots.
- `nk_plot_function`.
- `nk_combo_callback`.
- `nk_color_picker` (the `nk_color_pick` form is included).
- The "value return" variants of knobs and sliders (the `&mut` forms are included).
- Window cursors.
- `nk_widget` manual layout.
- Any render backend. The `demo` example draws with a small CPU rasterizer that lives in the example file, not in the crates.

## License

MIT OR Apache-2.0.
