mod app;
mod model;
mod pdf; // Required so the compiler recognizes src/pdf/
mod ui; // Required so the compiler recognizes src/ui/

use app::PdfArrangerApp;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("PDF Arranger")
            .with_inner_size([1120.0, 760.0])
            .with_min_inner_size([720.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native(
        "PDF Arranger",
        options,
        Box::new(|cc| Ok(Box::new(PdfArrangerApp::new(cc)))),
    )
}
