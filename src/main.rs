mod app;
mod model;
mod pdf; // Required so the compiler recognizes src/pdf/
mod ui; // Required so the compiler recognizes src/ui/

use app::PdfArrangerApp;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "PDF Arranger",
        options,
        Box::new(|cc| Ok(Box::new(PdfArrangerApp::new(cc)))),
    )
}
