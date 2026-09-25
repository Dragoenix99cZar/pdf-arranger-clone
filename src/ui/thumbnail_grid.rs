use crate::app::PdfArrangerApp;
use eframe::egui;

pub fn show_thumbnail_grid(app: &mut PdfArrangerApp, ui: &mut egui::Ui) {
    if app.project.pages.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label("No PDF loaded. Click 'Open PDF' or drag & drop a file here.");
        });
        return;
    }

    // Handle global keyboard shortcuts for page manipulation
    let mut delete_requested = false;
    let mut duplicate_requested = false;
    let mut move_delta: isize = 0;

    ui.input(|i| {
        if i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace) {
            delete_requested = true;
        }
        if i.modifiers.command && i.key_pressed(egui::Key::D) {
            duplicate_requested = true;
        }
        if i.modifiers.command && i.key_pressed(egui::Key::ArrowLeft) {
            move_delta = -1;
        }
        if i.modifiers.command && i.key_pressed(egui::Key::ArrowRight) {
            move_delta = 1;
        }
    });

    if delete_requested && !app.selected_ids.is_empty() {
        app.project.delete_pages(&app.selected_ids);
        app.selected_ids.clear();
        app.status_message = "Deleted selected pages.".to_string();
    }

    if duplicate_requested && !app.selected_ids.is_empty() {
        let new_ids = app.project.duplicate_pages(&app.selected_ids);
        app.selected_ids = new_ids;
        app.status_message = "Duplicated selected pages.".to_string();
    }

    if move_delta != 0 && !app.selected_ids.is_empty() {
        app.project.move_pages(&app.selected_ids, move_delta);
        app.status_message = "Reordered pages.".to_string();
    }

    let cols = app.grid_columns.max(1);
    let thumb_width = app.thumbnail_width;
    let show_preview = thumb_width > 40.0;
    let thumb_height = if show_preview {
        thumb_width * 1.4
    } else {
        20.0
    };

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

                        // Only load/fetch preview textures if width > 40
                        let texture_handle = if show_preview {
                            if let Some(texture) = app.texture_cache.get(&page.id) {
                                Some(texture.clone())
                            } else {
                                app.status_message = format!("Processing page: {}", page.source_page);
                                let pdf_path = app.project.source_files[page.source_document].clone();

                                let img = app
                                    .renderer
                                    .render_page_thumbnail(&pdf_path, page.source_page, 600)
                                    .unwrap_or_else(|_| image::DynamicImage::new_rgb8(100, 140));

                                let size = [img.width() as usize, img.height() as usize];
                                let image_buffer = img.to_rgba8();
                                let pixels = image_buffer.as_flat_samples();
                                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                                    size,
                                    pixels.as_slice(),
                                );

                                let texture = ui.ctx().load_texture(
                                    format!("thumb-{}", page.id),
                                    color_image,
                                    egui::TextureOptions::default(),
                                );

                                app.texture_cache.insert(page.id, texture.clone());
                                Some(texture)
                            }
                        } else {
                            None
                        };

                        let inner_response = egui::Frame::new()
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
                            .inner_margin(if show_preview { 6.0 } else { 2.0 })
                            .corner_radius(4.0)
                            .show(ui, |ui| {
                                ui.set_width(thumb_width);
                                ui.vertical_centered(|ui| {
                                    if let Some(texture) = texture_handle {
                                        ui.image((
                                            texture.id(),
                                            egui::vec2(thumb_width, thumb_height),
                                        ));
                                        ui.add_space(4.0);
                                        ui.label(format!("Page {}", page.source_page));
                                    } else {
                                        // Compact text-only view when width == 40
                                        ui.label(format!("{}", page.source_page));
                                    }
                                });
                            });

                        // Explicitly add click sensing to the card's bounding rectangle
                        let response = ui.interact(
                            inner_response.response.rect,
                            ui.id().with(page.id),
                            egui::Sense::click(),
                        );

                        // Context menu for right-click page actions
                        response.context_menu(|ui| {
                            if ui.button("Duplicate").clicked() {
                                if !app.selected_ids.contains(&page.id) {
                                    app.selected_ids.clear();
                                    app.selected_ids.insert(page.id);
                                }
                                app.selected_ids = app.project.duplicate_pages(&app.selected_ids);
                                app.status_message = "Duplicated selected pages.".to_string();
                                ui.close();
                            }
                            if ui.button("Move Left / Up").clicked() {
                                if !app.selected_ids.contains(&page.id) {
                                    app.selected_ids.clear();
                                    app.selected_ids.insert(page.id);
                                }
                                app.project.move_pages(&app.selected_ids, -1);
                                app.status_message = "Moved pages left.".to_string();
                                ui.close();
                            }
                            if ui.button("Move Right / Down").clicked() {
                                if !app.selected_ids.contains(&page.id) {
                                    app.selected_ids.clear();
                                    app.selected_ids.insert(page.id);
                                }
                                app.project.move_pages(&app.selected_ids, 1);
                                app.status_message = "Moved pages right.".to_string();
                                ui.close();
                            }
                            ui.separator();
                            if ui.button("Invert Selection").clicked() {
                                let mut inverted_ids = std::collections::HashSet::new();
                                for p in &app.project.pages {
                                    if !app.selected_ids.contains(&p.id) {
                                        inverted_ids.insert(p.id);
                                    }
                                }
                                app.selected_ids = inverted_ids;
                                app.status_message = "Inverted selection.".to_string();
                                ui.close();
                            }
                            ui.separator();
                            if ui.button("Delete").clicked() {
                                if !app.selected_ids.contains(&page.id) {
                                    app.selected_ids.clear();
                                    app.selected_ids.insert(page.id);
                                }
                                app.project.delete_pages(&app.selected_ids);
                                app.selected_ids.clear();
                                app.status_message = "Deleted selected pages.".to_string();
                                ui.close();
                            }
                        });

                        if response.clicked() {
                            let modifiers = ui.input(|i| i.modifiers);

                            if modifiers.command {
                                // Ctrl + Left Click: Un/Select multiple files individually
                                if is_selected {
                                    app.selected_ids.remove(&page.id);
                                } else {
                                    app.selected_ids.insert(page.id);
                                }
                                app.last_clicked_id = Some(page.id);
                            } else if modifiers.shift {
                                // Shift + Left Click: Select range from anchor to current page
                                let current_idx = app.project.pages
                                    .iter()
                                    .position(|p| p.id == page.id)
                                    .unwrap_or(0);

                                let anchor_idx = if let Some(last_id) = app.last_clicked_id {
                                    app.project.pages
                                        .iter()
                                        .position(|p| p.id == last_id)
                                        .unwrap_or(current_idx)
                                } else {
                                    current_idx
                                };

                                let start = std::cmp::min(anchor_idx, current_idx);
                                let end = std::cmp::max(anchor_idx, current_idx);

                                for idx in start..=end {
                                    if let Some(p) = app.project.pages.get(idx) {
                                        app.selected_ids.insert(p.id);
                                    }
                                }
                            } else {
                                // Left-click: Un/Select only the clicked file
                                app.selected_ids.clear();
                                app.selected_ids.insert(page.id);
                                app.last_clicked_id = Some(page.id);
                            }

                            // Debug print to console
                            let pdf_path = &app.project.source_files[page.source_document];
                            println!(
                                "Clicked Page ID: {} | Source Page: {} | File: {:?} | Total Selected: {}",
                                page.id,
                                page.source_page,
                                pdf_path,
                                app.selected_ids.len()
                            );
                        }
                    }
                });
            }
        });
}
