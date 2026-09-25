use crate::model::project::Project;
use crate::pdf::renderer::PdfRenderer;
use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use uuid::Uuid;

pub struct PdfArrangerApp {
    pub project: Project,
    pub renderer: PdfRenderer,
    pub selected_ids: HashSet<Uuid>,
    pub status_message: String,
    pub texture_cache: HashMap<Uuid, egui::TextureHandle>,
    pub grid_columns: usize,
    pub thumbnail_width: f32,
    pub last_clicked_id: Option<Uuid>,
}

impl Default for PdfArrangerApp {
    fn default() -> Self {
        Self {
            project: Project::default(),
            renderer: PdfRenderer::default(),
            selected_ids: HashSet::new(),
            status_message: "Ready. Open a PDF or drag and drop one here.".to_string(),
            texture_cache: HashMap::new(),
            grid_columns: 4,
            thumbnail_width: 100.0,
            last_clicked_id: None,
        }
    }
}

const MAX_COLUMN: usize = 15;

impl PdfArrangerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    pub fn load_pdf_file(&mut self, path: PathBuf) {
        self.status_message = format!(
            "Loading {}...",
            path.file_name().unwrap_or_default().to_string_lossy()
        );

        match lopdf::Document::load(&path) {
            Ok(doc) => {
                let page_count = doc.get_pages().len() as u32;
                self.project.add_source_file(path.clone(), page_count);
                self.status_message = format!(
                    "Loaded: {} ({} pages)",
                    path.file_name().unwrap_or_default().to_string_lossy(),
                    page_count
                );
            }
            Err(e) => {
                self.status_message = format!("Failed to load PDF: {}", e);
            }
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
        // 1. Check for Drag-and-Drop files
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                if let Some(file) = i.raw.dropped_files.first() {
                    if let Some(path) = &file.path {
                        self.load_pdf_file(path.clone());
                    }
                }
            }
        });

        // 2. Top Toolbar Panel
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open PDF...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("PDF Documents", &["pdf"])
                        .pick_file()
                    {
                        self.load_pdf_file(path);
                    }
                }

                if ui.button("Export PDF...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("PDF Documents", &["pdf"])
                        .save_file()
                    {
                        self.export_pdf_file(path);
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(
                        egui::Slider::new(&mut self.grid_columns, 1..=MAX_COLUMN).text("Columns"),
                    );
                    ui.separator();

                    let size_label = format!("Size: {:.0}", self.thumbnail_width);
                    ui.add(
                        egui::Slider::new(&mut self.thumbnail_width, 40.0..=400.0).text(size_label),
                    );
                });
            });
        });

        // 3. Status Bar Panel
        egui::TopBottomPanel::bottom("status_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status_message);
            });
        });

        // 4. Central Thumbnail Grid Panel
        egui::CentralPanel::default()
            .frame(egui::Frame::new().inner_margin(0.0))
            .show(ctx, |ui| {
                crate::ui::thumbnail_grid::show_thumbnail_grid(self, ui);
            });
    }
}
