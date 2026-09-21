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
