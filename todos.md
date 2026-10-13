# Todo
---

### Completed Milestones & Enhancements
* [x] **Milestone 1: File Loading & Ingestion**
  * Native file dialogs with multi-file support (`rfd::FileDialog::pick_files()`).
  * Multi-file drag-and-drop batch ingestion.
* [x] **Milestone 2: PDF Rendering Engine**
  * `PdfRenderer` module powered by `pdfium-render` and dynamic library linking.
  * Dedicated asynchronous `ThumbnailWorker` using `crossbeam-channel` on a background thread for non-blocking thumbnail generation.
  * Reactive UI updates via `ctx.request_repaint()`.
* [x] **Milestone 3: UI Thumbnail Grid & Layout**
  * Interactive grid supporting single selection, `Ctrl` multi-select, and `Shift` range selection.
  * Async texture caching and skeleton placeholder cards during background loading.
  * Real-time status bar spinner and pending thumbnail counter.
  * Dynamic column slider (1–15) and zoom slider (40–1200px) with compact text-only view at minimum size.
  * Multi-file thumbnail identification: displays `number1(Page number2)` (e.g. `1(Page 1)`, `2(Page 4)`) with source filename hover tooltips.
  * Modern empty state dropzone with drag-hover highlights and quick browse action.
* [x] **Milestone 4: Page & Document Manipulation**
  * Page reordering via `Ctrl+Left` / `Ctrl+Right` and context menu.
  * Page deletion via `Delete` / `Backspace` and context menu.
  * Page duplication via `Ctrl+D` and context menu.
  * Invert selection command.
* [x] **Milestone 5: PDF Export & Optimization**
  * Non-destructive recursive PDF export via `lopdf`.
  * **File Size Optimization**: Cross-page resource deduplication (`visited_per_doc`) preventing duplicate copies of shared fonts, images, and streams.
  * Object pruning (`prune_objects`) and stream compression (`compress`).
  * **Table of Contents & Bookmarks (`/Outlines`)**: Full extraction, destination remapping (explicit, GoTo actions, named destinations), and orphaned bookmark pruning.
  * Sets `/PageMode /UseOutlines` so PDF viewers automatically open the bookmarks sidebar.

### Upcoming Implementation Roadmap
* [ ] **Milestone 6: Page Rotation**
  * Support 90°, 180°, and 270° clockwise & counter-clockwise rotations.
  * Live visual thumbnail rotation preview.
  * Export rotation via `/Rotate` page dictionary entry.
  * Keyboard shortcuts (`Ctrl+R`, `Ctrl+Shift+R`) and context menu actions.
* [ ] **Milestone 7: Undo / Redo Command History**
  * Command pattern / history stack for layout operations (`Ctrl+Z`, `Ctrl+Y`).
* [ ] **Milestone 8: Drag-and-Drop Reordering in Grid**
  * Interactive mouse drag-and-drop of thumbnail cards directly inside the UI grid.
