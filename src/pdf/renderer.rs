use pdfium_render::prelude::*;
use std::path::Path;

pub struct PdfRenderer {
    pdfium: Pdfium,
}

impl Default for PdfRenderer {
    fn default() -> Self {
        let pdfium = Pdfium::new(
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
                .or_else(|_| Pdfium::bind_to_system_library())
                .unwrap_or_else(|e| panic!("Failed to load PDFium dynamic library: {}", e)),
        );
        Self { pdfium }
    }
}

impl PdfRenderer {
    pub fn render_page_thumbnail(
        &self,
        pdf_path: &Path,
        page_number: u32,
        max_dimension: u16,
    ) -> Result<image::DynamicImage, Box<dyn std::error::Error>> {
        let document = self.pdfium.load_pdf_from_file(pdf_path, None)?;

        // Convert page number to 0-based index and cast to i32
        let page_index = (page_number - 1) as i32;
        let page = document.pages().get(page_index)?;

        let render_config = PdfRenderConfig::new()
            .set_target_width(max_dimension as i32)
            .set_maximum_height(max_dimension as i32);

        let bitmap = page.render_with_config(&render_config)?;
        let image = bitmap.as_image()?;

        Ok(image)
    }
}

pub struct ThumbnailRequest {
    pub page_id: uuid::Uuid,
    pub pdf_path: std::path::PathBuf,
    pub page_number: u32,
    pub max_dimension: u16,
}

pub struct ThumbnailResponse {
    pub page_id: uuid::Uuid,
    pub image: eframe::egui::ColorImage,
}

pub struct ThumbnailWorker {
    request_tx: crossbeam_channel::Sender<ThumbnailRequest>,
    response_rx: crossbeam_channel::Receiver<ThumbnailResponse>,
}

impl ThumbnailWorker {
    pub fn new(ctx: eframe::egui::Context) -> Self {
        let (req_tx, req_rx) = crossbeam_channel::unbounded::<ThumbnailRequest>();
        let (res_tx, res_rx) = crossbeam_channel::unbounded::<ThumbnailResponse>();

        std::thread::Builder::new()
            .name("pdf-thumbnail-worker".to_string())
            .spawn(move || {
                let renderer = PdfRenderer::default();
                while let Ok(req) = req_rx.recv() {
                    if let Ok(img) = renderer.render_page_thumbnail(
                        &req.pdf_path,
                        req.page_number,
                        req.max_dimension,
                    ) {
                        let size = [img.width() as usize, img.height() as usize];
                        let image_buffer = img.to_rgba8();
                        let pixels = image_buffer.as_flat_samples();
                        let color_image = eframe::egui::ColorImage::from_rgba_unmultiplied(
                            size,
                            pixels.as_slice(),
                        );
                        let _ = res_tx.send(ThumbnailResponse {
                            page_id: req.page_id,
                            image: color_image,
                        });
                        ctx.request_repaint();
                    }
                }
            })
            .expect("Failed to spawn background thumbnail worker thread");

        Self {
            request_tx: req_tx,
            response_rx: res_rx,
        }
    }

    pub fn request(&self, req: ThumbnailRequest) {
        let _ = self.request_tx.send(req);
    }

    pub fn try_recv(&self) -> Result<ThumbnailResponse, crossbeam_channel::TryRecvError> {
        self.response_rx.try_recv()
    }
}
