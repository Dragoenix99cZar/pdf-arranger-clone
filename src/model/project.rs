use std::collections::HashSet;
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

        for page_num in 1..=page_count {
            self.pages.push(PageItem {
                id: Uuid::new_v4(),
                source_document: doc_index,
                source_page: page_num,
            });
        }
    }

    pub fn delete_pages(&mut self, selected_ids: &HashSet<Uuid>) {
        self.pages.retain(|p| !selected_ids.contains(&p.id));
    }

    pub fn duplicate_pages(&mut self, selected_ids: &HashSet<Uuid>) -> HashSet<Uuid> {
        let mut new_ids = HashSet::new();
        let mut new_pages = Vec::new();

        for page in &self.pages {
            new_pages.push(page.clone());
            if selected_ids.contains(&page.id) {
                let new_page = PageItem {
                    id: Uuid::new_v4(),
                    source_document: page.source_document,
                    source_page: page.source_page,
                };
                new_ids.insert(new_page.id);
                new_pages.push(new_page);
            }
        }
        self.pages = new_pages;
        new_ids
    }

    pub fn move_pages(&mut self, selected_ids: &HashSet<Uuid>, delta: isize) {
        if self.pages.is_empty() || selected_ids.is_empty() {
            return;
        }

        if delta < 0 {
            for i in 0..self.pages.len() {
                if selected_ids.contains(&self.pages[i].id) && i > 0 {
                    if !selected_ids.contains(&self.pages[i - 1].id) {
                        self.pages.swap(i, i - 1);
                    }
                }
            }
        } else if delta > 0 {
            for i in (0..self.pages.len()).rev() {
                if selected_ids.contains(&self.pages[i].id) && i + 1 < self.pages.len() {
                    if !selected_ids.contains(&self.pages[i + 1].id) {
                        self.pages.swap(i, i + 1);
                    }
                }
            }
        }
    }
}
