# Fractal agent guide

Fractal is a Rust Matrix client built with GTK4 and libadwaita. Its interface
templates use Blueprint (`.blp`) files and its build system is Meson.

## Working conventions

- Keep changes narrowly scoped to the requested task. Do not overwrite or
  discard existing worktree changes.
- Follow the surrounding Rust style and keep dependency lists sorted when they
  are changed.
- Update the corresponding Blueprint template when changing GTK UI structure,
  and keep Rust template bindings in sync.
- Prefer existing utilities and components over adding duplicate abstractions.
- Do not commit, push, or open pull requests unless explicitly requested.

## Validation

Run the smallest relevant check after a change. Common checks include:

```sh
cargo check
cargo test
```

Some checks require GTK, GStreamer, and other system dependencies; report a
missing dependency rather than attempting unrelated environment changes.
