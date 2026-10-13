use crate::model::project::{PageItem, Project};
use crate::pdf::renderer::{ThumbnailRequest, ThumbnailWorker};
use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use uuid::Uuid;

pub struct PdfArrangerApp {
    pub project: Project,
    pub selected_ids: HashSet<Uuid>,
    pub status_message: String,
    pub texture_cache: HashMap<Uuid, egui::TextureHandle>,
    pub grid_columns: usize,
    pub thumbnail_width: f32,
    pub last_clicked_id: Option<Uuid>,
    pub pending_thumbnails: HashSet<Uuid>,
    pub thumbnail_worker: Option<ThumbnailWorker>,
}

impl Default for PdfArrangerApp {
    fn default() -> Self {
        Self {
            project: Project::default(),
            selected_ids: HashSet::new(),
            status_message: "Ready. Open PDF(s) or drag and drop them here.".to_string(),
            texture_cache: HashMap::new(),
            grid_columns: 4,
            thumbnail_width: 100.0,
            last_clicked_id: None,
            pending_thumbnails: HashSet::new(),
            thumbnail_worker: None,
        }
    }
}

const MAX_COLUMN: usize = 15;

impl PdfArrangerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::from_rgb(22, 24, 30);
        visuals.window_fill = egui::Color32::from_rgb(26, 28, 35);
        visuals.faint_bg_color = egui::Color32::from_rgb(32, 35, 44);
        visuals.extreme_bg_color = egui::Color32::from_rgb(16, 17, 22);
        visuals.code_bg_color = egui::Color32::from_rgb(28, 30, 38);
        cc.egui_ctx.set_visuals(visuals);

        let mut app = Self::default();
        app.thumbnail_worker = Some(ThumbnailWorker::new(cc.egui_ctx.clone()));
        app
    }

    pub fn ensure_worker(&mut self, ctx: &egui::Context) -> &ThumbnailWorker {
        if self.thumbnail_worker.is_none() {
            self.thumbnail_worker = Some(ThumbnailWorker::new(ctx.clone()));
        }
        self.thumbnail_worker.as_ref().unwrap()
    }

    pub fn request_thumbnail(&mut self, page: &PageItem) {
        if self.texture_cache.contains_key(&page.id) || self.pending_thumbnails.contains(&page.id)
        {
            return;
        }

        if let Some(pdf_path) = self.project.source_files.get(page.source_document) {
            if let Some(worker) = &self.thumbnail_worker {
                self.pending_thumbnails.insert(page.id);
                worker.request(ThumbnailRequest {
                    page_id: page.id,
                    pdf_path: pdf_path.clone(),
                    page_number: page.source_page,
                    max_dimension: 1000,         // Thumbnail image quality/size
                });
            }
        }
    }

    #[allow(dead_code)]
    pub fn load_pdf_file(&mut self, path: PathBuf) {
        self.load_pdf_files(vec![path]);
    }

    pub fn load_pdf_files(&mut self, paths: Vec<PathBuf>) {
        if paths.is_empty() {
            return;
        }

        let mut loaded_files = 0;
        let mut total_new_pages = 0;
        let mut errors = Vec::new();
        let mut newly_added_pages = Vec::new();

        for path in paths {
            match lopdf::Document::load(&path) {
                Ok(doc) => {
                    let page_count = doc.get_pages().len() as u32;
                    let pages = self.project.add_source_file(path.clone(), page_count);
                    total_new_pages += page_count;
                    loaded_files += 1;
                    newly_added_pages.extend(pages);
                }
                Err(e) => {
                    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                    errors.push(format!("{}: {}", file_name, e));
                }
            }
        }

        // Queue newly added pages for background thumbnail generation
        for page in &newly_added_pages {
            self.request_thumbnail(page);
        }

        if !errors.is_empty() {
            self.status_message = format!(
                "Loaded {} file(s) ({} pages). Errors: {}",
                loaded_files,
                total_new_pages,
                errors.join("; ")
            );
        } else if loaded_files > 1 {
            self.status_message = format!(
                "Loaded {} files ({} pages total). Generating thumbnails in background...",
                loaded_files, total_new_pages
            );
        } else if loaded_files == 1 {
            self.status_message = format!(
                "Loaded {} pages. Generating thumbnails in background...",
                total_new_pages
            );
        }
    }

    pub fn export_pdf_file(&mut self, path: PathBuf) {
        self.status_message = format!(
            "Exporting to {}...",
            path.file_name().unwrap_or_default().to_string_lossy()
        );

        match crate::pdf::exporter::export_project(&self.project, &path) {
            Ok(()) => {
                self.status_message = format!(
                    "Successfully exported to {}",
                    path.file_name().unwrap_or_default().to_string_lossy()
                );
            }
            Err(e) => {
                self.status_message = format!("Failed to export PDF: {}", e);
            }
        }
    }
}

impl eframe::App for PdfArrangerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Ensure background worker thread is active
        self.ensure_worker(ctx);

        // Drain completed background thumbnails into texture cache
        let mut received_any = false;
        if let Some(worker) = &self.thumbnail_worker {
            while let Ok(resp) = worker.try_recv() {
                received_any = true;
                self.pending_thumbnails.remove(&resp.page_id);

                // Only upload and cache if the page is still in the active project
                if self.project.pages.iter().any(|p| p.id == resp.page_id) {
                    let texture = ctx.load_texture(
                        format!("thumb-{}", resp.page_id),
                        resp.image,
                        egui::TextureOptions::default(),
                    );
                    self.texture_cache.insert(resp.page_id, texture);
                }
            }
        }

        if received_any && self.pending_thumbnails.is_empty() {
            if self.status_message.contains("Generating thumbnails") {
                self.status_message = "Ready.".to_string();
            }
        }

        // 1. Check for Drag-and-Drop files (supports multiple PDFs at once)
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                let paths: Vec<PathBuf> = i
                    .raw
                    .dropped_files
                    .iter()
                    .filter_map(|f| f.path.clone())
                    .filter(|p| {
                        p.extension()
                            .and_then(|ext| ext.to_str())
                            .map_or(false, |ext| ext.eq_ignore_ascii_case("pdf"))
                    })
                    .collect();

                if !paths.is_empty() {
                    self.load_pdf_files(paths);
                }
            }
        });

        // 2. Top Toolbar Panel
        egui::TopBottomPanel::top("top_panel")
            .frame(
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(22, 24, 30))
                    .inner_margin(8.0),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);

                    if ui.button("📂 Open PDF(s)...").clicked() {
                        if let Some(paths) = rfd::FileDialog::new()
                            .add_filter("PDF Documents", &["pdf"])
                            .pick_files()
                        {
                            self.load_pdf_files(paths);
                        }
                    }

                    let has_pages = !self.project.pages.is_empty();
                    ui.add_enabled_ui(has_pages, |ui| {
                        if ui.button("💾 Export PDF...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("PDF Documents", &["pdf"])
                                .save_file()
                            {
                                self.export_pdf_file(path);
                            }
                        }
                    });

                    if has_pages {
                        ui.separator();
                        ui.label(
                            egui::RichText::new(format!("📑 {} pages", self.project.pages.len()))
                                .color(egui::Color32::from_rgb(170, 175, 195))
                                .size(12.5),
                        );
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().slider_width = 80.0;
                        ui.add(
                            egui::Slider::new(&mut self.grid_columns, 1..=MAX_COLUMN)
                                .text("Columns"),
                        );
                        ui.separator();

                        ui.add(
                            egui::Slider::new(&mut self.thumbnail_width, 40.0..=1200.0)
                                .show_value(true)
                                .text("Zoom"),
                        );
                    });
                });
            });

        // 3. Status Bar Panel
        egui::TopBottomPanel::bottom("status_panel")
            .frame(
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(22, 24, 30))
                    .inner_margin(6.0),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&self.status_message)
                            .color(egui::Color32::from_rgb(160, 165, 180))
                            .size(12.0),
                    );
                    if !self.pending_thumbnails.is_empty() {
                        ui.separator();
                        ui.spinner();
                        ui.label(
                            egui::RichText::new(format!(
                                "Loading thumbnails ({} pending)...",
                                self.pending_thumbnails.len()
                            ))
                            .color(egui::Color32::from_rgb(140, 145, 160))
                            .size(12.0),
                        );
                    }
                });
            });

        // 4. Central Thumbnail Grid Panel
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(17, 18, 23))
                    .inner_margin(0.0),
            )
            .show(ctx, |ui| {
                crate::ui::thumbnail_grid::show_thumbnail_grid(self, ui);
            });
    }
}
