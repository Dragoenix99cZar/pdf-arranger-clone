use crate::model::project::Project;
use lopdf::{Dictionary, Document, Object, ObjectId};
use std::path::Path;

pub fn export_project(
    project: &Project,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if project.pages.is_empty() {
        return Err("No pages to export in the project.".into());
    }

    // Load all source documents referenced in the project
    let mut source_docs = Vec::new();
    for path in &project.source_files {
        let doc = Document::load(path)?;
        source_docs.push(doc);
    }

    // Create a new destination PDF document
    let mut dest_doc = Document::with_version("1.5");
    let mut cloned_page_ids = Vec::new();

    for page_item in &project.pages {
        let src_doc = &source_docs[page_item.source_document];
        let pages_map = src_doc.get_pages();

        if let Some(&src_page_id) = pages_map.get(&page_item.source_page) {
            // Recursively copy the page object and all its dependencies (resources, contents streams, etc.)
            let new_page_id = copy_object_recursively(src_doc, &mut dest_doc, src_page_id);
            cloned_page_ids.push(new_page_id);
        }
    }

    // Create Pages root and Catalog root for dest_doc
    let pages_id = dest_doc.new_object_id();
    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", Object::Name(b"Pages".to_vec()));

    let kids: Vec<Object> = cloned_page_ids
        .iter()
        .map(|&id| Object::Reference(id))
        .collect();
    pages_dict.set("Kids", Object::Array(kids));
    pages_dict.set("Count", Object::Integer(cloned_page_ids.len() as i64));

    dest_doc
        .objects
        .insert(pages_id, Object::Dictionary(pages_dict));

    // Update each page's "Parent" reference to point to pages_id
    for &page_id in &cloned_page_ids {
        if let Ok(Object::Dictionary(dict)) = dest_doc.get_object_mut(page_id) {
            dict.set("Parent", Object::Reference(pages_id));
        }
    }

    let catalog_id = dest_doc.new_object_id();
    let mut catalog_dict = Dictionary::new();
    catalog_dict.set("Type", Object::Name(b"Catalog".to_vec()));
    catalog_dict.set("Pages", Object::Reference(pages_id));
    dest_doc
        .objects
        .insert(catalog_id, Object::Dictionary(catalog_dict));

    dest_doc.trailer.set("Root", Object::Reference(catalog_id));
    dest_doc.max_id = dest_doc.objects.len() as u32;

    // Save the generated document to the target path
    dest_doc.save(output_path)?;

    Ok(())
}

/// Helper function to recursively copy an object and its references from source to destination document
fn copy_object_recursively(
    src_doc: &Document,
    dest_doc: &mut Document,
    obj_id: ObjectId,
) -> ObjectId {
    let obj = match src_doc.get_object(obj_id) {
        Ok(o) => o,
        Err(_) => return obj_id,
    };

    let new_id = dest_doc.new_object_id();

    // Insert a placeholder first to handle potential circular references gracefully
    match obj {
        Object::Dictionary(_) => {
            dest_doc
                .objects
                .insert(new_id, Object::Dictionary(Dictionary::new()));
        }
        Object::Array(_) => {
            dest_doc.objects.insert(new_id, Object::Array(Vec::new()));
        }
        _ => {
            dest_doc.objects.insert(new_id, obj.clone());
            return new_id;
        }
    }

    let cloned_obj = map_object_references(src_doc, dest_doc, obj);
    dest_doc.objects.insert(new_id, cloned_obj);
    new_id
}

/// Maps and copies nested objects/references
fn map_object_references(src_doc: &Document, dest_doc: &mut Document, obj: &Object) -> Object {
    match obj {
        Object::Reference(ref_id) => {
            // Recursively copy referenced objects (like Contents or Resources streams/dictionaries)
            let new_ref_id = copy_object_recursively(src_doc, dest_doc, *ref_id);
            Object::Reference(new_ref_id)
        }
        Object::Dictionary(dict) => {
            let mut new_dict = Dictionary::new();
            for (key, value) in dict.iter() {
                // Skip the Parent key so it doesn't point back to the old document's Pages root tree
                if key == b"Parent" {
                    continue;
                }
                new_dict.set(key.clone(), map_object_references(src_doc, dest_doc, value));
            }
            Object::Dictionary(new_dict)
        }
        Object::Array(arr) => {
            let new_arr = arr
                .iter()
                .map(|v| map_object_references(src_doc, dest_doc, v))
                .collect();
            Object::Array(new_arr)
        }
        _ => obj.clone(),
    }
}
