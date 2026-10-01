# Phase 0: desktop integration

Implementation is ready to run on macOS. The macOS launch/Retina/input checks passed locally; Windows/Linux execution and several hardware/IME checks below remain pending. This is an integration spike, not the Phase 1 document engine.

## Decisions fixed by this spike

- **Shell:** egui 0.36.2 + egui-winit 0.36.2 + egui-wgpu 0.36.2, winit 0.30.13, WGPU 30.0.1 (locked). Rust 1.95 minimum from dependencies; built here with Rust 1.98.1.
- **GPU ownership:** the desktop app creates one instance, surface, device and queue. fox-graphics and egui receive the same device and draw successive passes into the same surface texture. No texture readback or second GPU device is used to compose the UI.
- **Graphics boundary:** fox-graphics 0.2 contains a straight-stroke renderer accepting logical-pixel primitives. It has no dependency on the GUI or window system. The earlier app builder, event loop, 3D camera and atlas prototypes were removed; fig is not kept compatible.
- **Precision:** world and camera math use f64. Screen-space line clipping happens before conversion to f32. Pan/zoom operate in logical pixels; the render viewport/scissor uses framebuffer pixels and egui's effective pixels-per-point.
- **Units:** initial drawing units are millimeters, decimals use a period, Cartesian coordinates are X right/Y up, polar angles use degrees counterclockwise from +X. Phase 1 should represent units as document metadata and initially support unitless/mm/cm/m/in/ft. Changing formatting never silently rescales geometry.
- **Point entry:** absolute `10,20`; relative `@10,20` or `@10` in X with `20` in Y; relative polar `@25<30`. Relative input needs a previous anchor. Separate fields accept `10`, Tab, `10`, Enter. Enter validates the group; incomplete groups remain open.
- **Prompt/focus:** GUI and typed LINE use the same session. Tab/Shift+Tab cycle X/Y; command-name Tab completes LINE. F6 leaves or returns to the command field. Pointer hover does not replace a typed draft. A click with a draft asks the user to submit/clear it. Ordinary field text entry is routed through egui/winit, not physical keycode translation.
- **LINE policy:** accepted segments stay when finishing/canceling; rubber-band preview never becomes accepted geometry by itself. Escape first clears an edited draft, then finishes the command. Enter with an empty group finishes after an anchor. Phase 1 will group the accepted segments into one undo item, with a separate command-local Undo step.
- **Selection specification for Phase 1:** preselection and command-requested selection share a service; click/overlap cycle; left-to-right window contains; right-to-left crosses; Shift removes from selection. Selection itself is session state. Hidden geometry is excluded; locked geometry remains available for inspection/snapping but cannot be edited. Selection is not implemented in this spike.
- **Command extension:** the toolkit-independent interaction model here is a prototype. The separate command API/provider registration and example external provider remain Phase 1 work. Do not grow this spike's single LINE dispatch into a permanent central enum/match dispatcher.

## Implemented layout

| File | Responsibility |
| --- | --- |
| `apps/foxcad/src/main.rs` | Main-thread winit lifecycle, GPU setup, egui integration, repaint scheduling and smoke mode |
| `apps/foxcad/src/ui.rs` | Command toolbar, tools/status, viewport interaction and drawing presentation |
| `apps/foxcad/src/command.rs` | GPU/GUI-independent temporary LINE state and coordinate parser |
| `apps/foxcad/src/viewport.rs` | f64 world/screen mapping, pan and pointer-anchored zoom |
| `../fox-graphics/src/lib.rs` | Reusable GPU triangle-based stroke renderer |
| `../fox-graphics/src/stroke.wgsl` | Stroke shader with derivative-based edge antialiasing |

The small spike deliberately occupies one app crate. Phase 1 extracts the production core, command API and editor boundaries from the architecture instead of scaffolding empty crates now.

## Lifecycle and rendering

The loop waits when there is no pending repaint. Input, resize, GUI animation/caret deadlines and GPU recovery schedule redraws. It does not request a new frame unconditionally. Zero-sized windows skip presentation. Surface timeout retries are delayed; outdated/suboptimal surfaces are reconfigured; lost surfaces are recreated. Device-loss handling rebuilds render resources and the egui font context while keeping the temporary drawing. These recovery paths are implemented but have not been fault-injection tested.

The stroke renderer currently draws independent segments with simple butt ends, no joins or dash pattern, into a scissored viewport. It is enough to validate precision/composition, not the final drafting renderer. The CPU rebuilds the small stroke list each frame; retained geometry caches belong to later work.

## Validation performed locally

- `cargo test --locked`: nine passing tests. They cover compact/separate coordinate equivalence, relative/polar parsing, partial input and Escape, accepted-segment retention, large-origin camera math, anchored zoom, real egui Tab/Shift+Tab focus behavior, and shared typed/button prompt presentation.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- Formatting checked for both FoxCAD and fox-graphics.
- `cargo run --locked -- --smoke-test`: passed, four frames presented on Apple M2 / Metal with Bgra8Unorm and 2.00 pixels per point. This creates actual GPU pipelines and a window, not a mocked renderer.
- Native development bundle launched and visually inspected. Confirmed LINE via typing and the toolbar; `10 → Tab → 10 → Enter` produced `(10,10)`; `@80,40` produced `(90,50)` and rendered an accepted segment; Escape finished without exiting the app. Command completion and wheel zoom also exercised.
- Corrected two integration issues discovered during runtime inspection: egui texture deltas must be acknowledged after upload/free; explicit dark theme is needed to keep platform theme detection from mismatching custom panel backgrounds.

## Outstanding checks and limits

- **Windows/Linux startup is not yet verified.** Only a macOS host is available in this workspace. `.github/workflows/phase0.yml` provides a manually dispatched three-platform test/build/window smoke matrix. It requires a published fox-graphics ref containing the new API; checking out its old main branch would not test this work. Neither repository was published and the workflow was not run here.
- Mixed-DPI monitor transitions, physical trackpad pinch, IME composition in a non-Latin input method, and device-loss recovery need hardware/manual validation. Winit/egui paths are connected; support is not claimed solely from that wiring.
- The accessibility tree currently exposes the native window, not the egui controls: AccessKit integration remains to be wired and tested before claiming screen-reader support. F6 avoids trapping keyboard users in the coordinate group.
- No native document, persistence, history/undo, selection, snap engine, plugin loader, dimensions, or font/export subsystem. All accepted segments are disposable memory-only data.
- The development `.app` is unsigned and intended for this local workspace; distribution packaging/signing is later work.

## Run and development bundle

From the FoxCAD root, `cargo run --locked` starts the app. Both sibling repositories are required. `scripts/bundle-macos.sh` builds `target/FoxCAD.app` for Finder/Dock launching; quit an old running instance before rebuilding/relaunching it. It is safe to close the spike because it contains only temporary test geometry.

To finish the cross-platform gate, publish coordinated FoxCAD/fox-graphics revisions, dispatch the workflow with that graphics ref, and record results here. Phase 1 should retain the validated shared-device integration and replace the temporary command/geometry storage with the specified host/provider and transaction model.
