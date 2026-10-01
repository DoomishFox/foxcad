# FoxCAD

Native Rust 2D drafting application. **Phase 0 integration spike**: egui command UI and a fox-graphics viewport sharing one WGPU device, queue, and surface.

## Run

Requires Rust 1.95 or newer and sibling checkouts of `foxcad` and the updated `fox-graphics` from this work. The old graphics API is not compatible.

```sh
cargo run --locked
```

- Click **Line**, or type `LINE` / `L` and Enter. Tab completes the command name.
- Enter X, Tab, Y, Enter; or enter `10,10` in X with Y empty.
- After an anchor, use `@10,20` or `@25<30` in X for relative or polar coordinates.
- Click to pick points when no numeric draft is pending.
- Enter with empty fields finishes LINE. Escape clears a draft first, then finishes.
- Middle/right drag pans. Scroll/pinch zooms at the pointer. Shift+scroll pans.
- F6 leaves/focuses the command toolbar. Reset view restores the initial camera.

Coordinates are millimeters in this spike. White segments are temporary accepted geometry; orange is the current preview. No save, undo, selection, object snapping, plugin loader or production document model exists yet. Closing the window discards the temporary drawing.

On macOS, `./scripts/bundle-macos.sh` also creates `target/FoxCAD.app` for Finder/Dock launching. Quit a running instance before rebuilding it.

## Verify

```sh
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo run --locked -- --smoke-test
```

The smoke test creates a real window and GPU pipelines, presents four frames, then exits. It fails after 30 seconds without completing. Linux needs a desktop session (or Xvfb and a software Vulkan driver); this is not a GPU-free test.

See [Phase 0 decisions and validation](docs/phase-0.md) and the [architecture](docs/architecture.md).
