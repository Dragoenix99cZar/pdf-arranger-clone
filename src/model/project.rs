use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PageItem {
    pub id: Uuid,
    pub source_document: usize,
    pub source_page: u32,
}

#[derive(Default, Debug)]
pub struct Project {
    pub source_files: Vec<PathBuf>,
    pub pages: Vec<PageItem>,
}

impl Project {
    pub fn add_source_file(&mut self, path: PathBuf, page_count: u32) {
        let doc_index = self.source_files.len();
        self.source_files.push(path);

        // Add a PageItem for each page in the opened PDF
        for page_num in 1..=page_count {
            self.pages.push(PageItem {
                id: Uuid::new_v4(),
                source_document: doc_index,
                source_page: page_num,
            });
        }
    }
}
