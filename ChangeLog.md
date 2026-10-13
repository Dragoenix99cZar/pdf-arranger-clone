# Changelog

All notable changes to the **pdf-arranger-clone** project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [Unreleased] - 2026-10-13

### Fixed
- **Export File Size Bloat**:
  - Resolved an issue where exporting an unmodified or arranged PDF resulted in an exported file significantly larger than the source document.
  - **Root Cause**: The visited object tracking map was previously instantiated inside the page loop per-page, causing shared resources (embedded fonts, Type0/TrueType/OpenType/CFF font streams, images/XObjects, color spaces) to be duplicated for every single page.
  - **Resolution**: Implemented persistent document-level tracking via `visited_per_doc` in `exporter.rs`, sharing deduplicated resources across all pages from the same source document while giving each page its own distinct page dictionary.
  - Added `dest_doc.prune_objects()` to eliminate unreachable/orphaned objects.
  - Added `dest_doc.compress()` to compress stream contents with `FlateDecode`.
- **Top Toolbar UI Collision & Overlap**:
  - Fixed an issue where the top toolbar controls (e.g., "Export PDF" and zoom/column sliders) overlapped and collided due to unconstrained slider widths and cramped default window dimensions.
  - Constrained slider widths, set clean labels (`Zoom`, `Columns`), and added disabled states for the Export button when no pages are loaded.

### Added
- **PDF Bookmarks & Table of Contents (`/Outlines`) Export**:
  - Implemented full extraction and reconstruction of PDF outline hierarchies across all imported source documents.
  - Supported explicit destinations (`/Dest [page /XYZ ...]`), GoTo actions (`/A << /S /GoTo /D ... >>`), and named destinations (resolved via `/Catalog /Dests` dictionaries and `/Names -> /Dests` name trees).
  - Implemented automatic target page remapping: outline destinations update dynamically to point to the newly arranged page positions in the output PDF.
  - Added pruning for bookmarks pointing to deleted pages that have no surviving child items, preserving outline tree integrity.
  - Added `/PageMode /UseOutlines` to the output Catalog so PDF viewers automatically open with the bookmarks/outline sidebar visible.
- **Multi-PDF File Batch Ingestion**:
  - Enabled multi-selection in the native file open dialog using `rfd::FileDialog::pick_files()`.
  - Upgraded drag-and-drop listener to collect and import all dropped `.pdf` files simultaneously.
  - Added batch loading routine `load_pdf_files` to ingest multiple source PDFs in sequence.
- **Asynchronous Background Thumbnail Processing**:
  - Introduced `ThumbnailWorker` running on a dedicated worker thread using `crossbeam-channel` and an isolated `PdfRenderer` instance.
  - Completely decoupled thumbnail rendering from the egui render loop, eliminating UI stutter, frame drops, and window freezes when loading large documents.
  - Added reactive UI updates via `ctx.request_repaint()` as background thumbnails complete.
  - Added lightweight skeleton placeholder cards while thumbnails are loading in the background.
  - Added real-time progress feedback in the status bar (spinner and pending thumbnail counter).
- **Modern Empty State & Dropzone Canvas**:
  - Replaced the blank, pitch-black startup screen with an interactive dropzone card featuring a crimson `📄 PDF` document badge, clear instructions, and feature highlight pills.
  - Added interactive drag-over hover detection: the dropzone card glows with an accent blue border (`#4285f4`) and background highlight when dragging files into the window.
  - Added a prominent primary CTA button ("📂 Browse PDF Files...") directly within the dropzone.
- **Multi-Document Thumbnail Notation**:
  - Added clear multi-file thumbnail labeling: when multiple documents are loaded, thumbnail titles display as `number1(Page number2)` (e.g. `1(Page 1)`..`1(Page 10)`, `2(Page 1)`..`2(Page 4)`), where `number1` indicates the 1-based source PDF index and `number2` indicates the page number.
  - Added hover tooltips on thumbnail cards displaying the source file name and page number.
- **Window Viewport Configuration**:
  - Configured native window options to open at a comfortable `1120 x 760` resolution with a minimum window size of `720 x 500`.

---

## [0.1.0] - 2026-09-25

### Added
- Initial Rust prototype using `eframe`/`egui`, `pdfium-render`, and `lopdf`.
- Milestone 1: PDF ingestion via file dialog and drag-and-drop.
- Milestone 2: PDFium page rasterization into GUI image textures.
- Milestone 3: Interactive scrollable thumbnail grid with single, `Ctrl` multi-select, and `Shift` range selection.
- Milestone 4: Page deletion (`Delete`/`Backspace`), duplication (`Ctrl+D`), and reordering (`Ctrl+Left/Right`).
- Milestone 5: Initial non-destructive PDF exporter preserving vector content and embedded fonts.
