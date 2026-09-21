use crate::app::PdfArrangerApp;
use eframe::egui;

pub fn show_thumbnail_grid(app: &mut PdfArrangerApp, ui: &mut egui::Ui) {
    if app.project.pages.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label("No PDF loaded. Click 'Open PDF' or drag & drop a file here.");
        });
        return;
    }

    let cols = app.grid_columns.max(1);
    let thumb_width = app.thumbnail_width; // Use dynamic thumbnail size from app state
    let thumb_height = thumb_width * 1.4;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(8.0);
            });

            ui.spacing_mut().item_spacing = egui::vec2(16.0, 16.0);

            let pages = app.project.pages.clone();

            for chunk in pages.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    for page in chunk {
                        let is_selected = app.selected_ids.contains(&page.id);

                        let texture_handle =
                            app.texture_cache.entry(page.id).or_insert_with(|| {
                                let pdf_path = &app.project.source_files[page.source_document];
                                let img = app
                                    .renderer
                                    .render_page_thumbnail(pdf_path, page.source_page, 600)
                                    .unwrap_or_else(|_| image::DynamicImage::new_rgb8(100, 140));

                                let size = [img.width() as usize, img.height() as usize];
                                let image_buffer = img.to_rgba8();
                                let pixels = image_buffer.as_flat_samples();
                                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                                    size,
                                    pixels.as_slice(),
                                );

                                ui.ctx().load_texture(
                                    format!("thumb-{}", page.id),
                                    color_image,
                                    egui::TextureOptions::default(),
                                )
                            });

                        let response = egui::Frame::new()
                            .fill(if is_selected {
                                ui.visuals().selection.bg_fill
                            } else {
                                ui.visuals().code_bg_color
                            })
                            .stroke(if is_selected {
                                egui::Stroke::new(2.0, ui.visuals().selection.stroke.color)
                            } else {
                                egui::Stroke::NONE
                            })
                            .inner_margin(6.0)
                            .corner_radius(4.0)
                            .show(ui, |ui| {
                                ui.set_width(thumb_width);
                                ui.vertical_centered(|ui| {
                                    ui.image((
                                        texture_handle.id(),
                                        egui::vec2(thumb_width, thumb_height),
                                    ));
                                    ui.add_space(4.0);
                                    ui.label(format!("Page {}", page.source_page));
                                });
                            })
                            .response;

                        if response.clicked() {
                            if ui.input(|i| i.modifiers.command || i.modifiers.shift) {
                                if is_selected {
                                    app.selected_ids.remove(&page.id);
                                } else {
                                    app.selected_ids.insert(page.id);
                                }
                            } else {
                                app.selected_ids.clear();
                                app.selected_ids.insert(page.id);
                            }
                        }
                    }
                });
            }
        });
}
