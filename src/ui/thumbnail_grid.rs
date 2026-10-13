use crate::app::PdfArrangerApp;
use eframe::egui;

pub fn show_thumbnail_grid(app: &mut PdfArrangerApp, ui: &mut egui::Ui) {
    if app.project.pages.is_empty() {
        show_empty_state(app, ui);
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
        for id in &app.selected_ids {
            app.texture_cache.remove(id);
            app.pending_thumbnails.remove(id);
        }
        app.project.delete_pages(&app.selected_ids);
        app.selected_ids.clear();
        app.status_message = "Deleted selected pages.".to_string();
    }

    if duplicate_requested && !app.selected_ids.is_empty() {
        let new_ids = app.project.duplicate_pages(&app.selected_ids);
        let duplicated_pages: Vec<_> = app
            .project
            .pages
            .iter()
            .filter(|p| new_ids.contains(&p.id))
            .cloned()
            .collect();
        for page in &duplicated_pages {
            app.request_thumbnail(page);
        }
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
                                app.request_thumbnail(page);
                                None
                            }
                        } else {
                            None
                        };

                        let multiple_files = app.project.source_files.len() > 1;
                        let page_label = if multiple_files {
                            format!("{}(Page {})", page.source_document + 1, page.source_page)
                        } else {
                            format!("Page {}", page.source_page)
                        };
                        let compact_label = if multiple_files {
                            format!("{}({})", page.source_document + 1, page.source_page)
                        } else {
                            format!("{}", page.source_page)
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
                                        ui.label(&page_label);
                                    } else if show_preview {
                                        let (rect, _) = ui.allocate_exact_size(
                                            egui::vec2(thumb_width, thumb_height),
                                            egui::Sense::hover(),
                                        );
                                        ui.painter().rect_filled(
                                            rect,
                                            4.0,
                                            ui.visuals().faint_bg_color,
                                        );
                                        ui.painter().text(
                                            rect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            &page_label,
                                            egui::FontId::proportional(12.0),
                                            ui.visuals().weak_text_color(),
                                        );
                                        ui.add_space(4.0);
                                        ui.label(&page_label);
                                    } else {
                                        // Compact text-only view when width <= 40
                                        ui.label(&compact_label);
                                    }
                                });
                            });

                        // Explicitly add click sensing to the card's bounding rectangle
                        let mut response = ui.interact(
                            inner_response.response.rect,
                            ui.id().with(page.id),
                            egui::Sense::click(),
                        );

                        if let Some(path) = app.project.source_files.get(page.source_document) {
                            let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                            if multiple_files {
                                response = response.on_hover_text(format!(
                                    "File #{}: {}\nPage {}",
                                    page.source_document + 1,
                                    file_name,
                                    page.source_page
                                ));
                            } else {
                                response = response.on_hover_text(format!("Page {}", page.source_page));
                            }
                        }

                        // Context menu for right-click page actions
                        response.context_menu(|ui| {
                            if ui.button("Duplicate").clicked() {
                                if !app.selected_ids.contains(&page.id) {
                                    app.selected_ids.clear();
                                    app.selected_ids.insert(page.id);
                                }
                                let new_ids = app.project.duplicate_pages(&app.selected_ids);
                                let duplicated_pages: Vec<_> = app
                                    .project
                                    .pages
                                    .iter()
                                    .filter(|p| new_ids.contains(&p.id))
                                    .cloned()
                                    .collect();
                                for p in &duplicated_pages {
                                    app.request_thumbnail(p);
                                }
                                app.selected_ids = new_ids;
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
                                for id in &app.selected_ids {
                                    app.texture_cache.remove(id);
                                    app.pending_thumbnails.remove(id);
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
                                let current_idx = app
                                    .project
                                    .pages
                                    .iter()
                                    .position(|p| p.id == page.id)
                                    .unwrap_or(0);

                                let anchor_idx = if let Some(last_id) = app.last_clicked_id {
                                    app.project
                                        .pages
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
                            // let pdf_path = &app.project.source_files[page.source_document];
                            // println!(
                            //     "Clicked Page ID: {} | Source Page: {} | File: {:?} | Total Selected: {}",
                            //     page.id,
                            //     page.source_page,
                            //     pdf_path,
                            //     app.selected_ids.len()
                            // );
                        }
                    }
                });
            }
        });
}

fn show_empty_state(app: &mut PdfArrangerApp, ui: &mut egui::Ui) {
    let is_hovered = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());

    ui.vertical_centered(|ui| {
        let total_height = ui.available_height();
        let desired_card_height = 360.0;
        let top_pad = ((total_height - desired_card_height) * 0.45).max(30.0);
        ui.add_space(top_pad);

        let card_width = 540.0;
        let border_color = if is_hovered {
            egui::Color32::from_rgb(66, 133, 244)
        } else {
            egui::Color32::from_rgb(50, 54, 66)
        };
        let bg_color = if is_hovered {
            egui::Color32::from_rgb(26, 34, 52)
        } else {
            egui::Color32::from_rgb(24, 26, 33)
        };

        egui::Frame::new()
            .fill(bg_color)
            .stroke(egui::Stroke::new(if is_hovered { 2.5 } else { 1.5 }, border_color))
            .corner_radius(16.0)
            .inner_margin(32.0)
            .show(ui, |ui| {
                ui.set_width(card_width);
                ui.vertical_centered(|ui| {
                    // Document Badge
                    egui::Frame::new()
                        .fill(egui::Color32::from_rgb(219, 58, 58))
                        .corner_radius(8.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.colored_label(
                                egui::Color32::WHITE,
                                egui::RichText::new("📄  PDF").strong().size(14.0),
                            );
                        });

                    ui.add_space(16.0);

                    if is_hovered {
                        ui.colored_label(
                            egui::Color32::from_rgb(66, 133, 244),
                            egui::RichText::new("Drop files to import now!")
                                .strong()
                                .size(22.0),
                        );
                    } else {
                        ui.colored_label(
                            egui::Color32::from_rgb(240, 243, 250),
                            egui::RichText::new("Drop PDF files here")
                                .strong()
                                .size(22.0),
                        );
                    }

                    ui.add_space(6.0);
                    ui.colored_label(
                        egui::Color32::from_rgb(140, 146, 165),
                        egui::RichText::new(
                            "Open single or multiple PDF documents to rearrange, merge, duplicate, or delete pages",
                        )
                        .size(13.0),
                    );

                    ui.add_space(24.0);

                    let open_btn = egui::Button::new(
                        egui::RichText::new("📂  Browse PDF Files...")
                            .size(15.0)
                            .strong()
                            .color(egui::Color32::WHITE),
                    )
                    .min_size(egui::vec2(220.0, 42.0))
                    .fill(egui::Color32::from_rgb(46, 100, 215))
                    .corner_radius(8.0);

                    if ui.add(open_btn).clicked() {
                        if let Some(paths) = rfd::FileDialog::new()
                            .add_filter("PDF Documents", &["pdf"])
                            .pick_files()
                        {
                            app.load_pdf_files(paths);
                        }
                    }

                    ui.add_space(26.0);
                    ui.separator();
                    ui.add_space(14.0);

                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(10.0, 8.0);
                        for tag in [
                            "✦ Multi-File Merge",
                            "✦ Drag & Drop",
                            "✦ Instant Preview",
                            "✦ Bookmarks Preserved",
                        ] {
                            egui::Frame::new()
                                .fill(egui::Color32::from_rgb(32, 35, 45))
                                .corner_radius(6.0)
                                .inner_margin(6.0)
                                .show(ui, |ui| {
                                    ui.colored_label(
                                        egui::Color32::from_rgb(175, 180, 200),
                                        egui::RichText::new(tag).size(11.5),
                                    );
                                });
                        }
                    });
                });
            });
    });
}
