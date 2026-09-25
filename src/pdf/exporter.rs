use crate::model::project::Project;
use lopdf::{Dictionary, Document, Object, ObjectId};
use std::collections::HashMap;
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
            // Keep track of visited object IDs per page to avoid duplicate copying cycles
            let mut visited = HashMap::new();
            let new_page_id = copy_object_deep(src_doc, &mut dest_doc, src_page_id, &mut visited);
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

/// Recursively copy an object, its deep dictionary fields, font descriptors, and stream buffers.
fn copy_object_deep(
    src_doc: &Document,
    dest_doc: &mut Document,
    obj_id: ObjectId,
    visited: &mut HashMap<ObjectId, ObjectId>,
) -> ObjectId {
    if let Some(&new_id) = visited.get(&obj_id) {
        return new_id;
    }

    let obj = match src_doc.get_object(obj_id) {
        Ok(o) => o,
        Err(_) => return obj_id,
    };

    let new_id = dest_doc.new_object_id();
    visited.insert(obj_id, new_id);

    let cloned_obj = match obj {
        Object::Dictionary(dict) => {
            let mut new_dict = Dictionary::new();
            for (key, value) in dict.iter() {
                // Skip the Parent link to avoid circular hierarchy tree bugs
                if key == b"Parent" {
                    continue;
                }
                new_dict.set(
                    key.clone(),
                    map_value_deep(src_doc, dest_doc, value, visited),
                );
            }
            Object::Dictionary(new_dict)
        }
        Object::Array(arr) => {
            let new_arr = arr
                .iter()
                .map(|v| map_value_deep(src_doc, dest_doc, v, visited))
                .collect();
            Object::Array(new_arr)
        }
        Object::Stream(stream) => {
            let mut new_dict = Dictionary::new();
            for (key, value) in stream.dict.iter() {
                new_dict.set(
                    key.clone(),
                    map_value_deep(src_doc, dest_doc, value, visited),
                );
            }
            let mut new_stream = stream.clone();
            new_stream.dict = new_dict;
            Object::Stream(new_stream)
        }
        _ => obj.clone(),
    };

    dest_doc.objects.insert(new_id, cloned_obj);
    new_id
}

/// Maps object references recursively across documents.
fn map_value_deep(
    src_doc: &Document,
    dest_doc: &mut Document,
    value: &Object,
    visited: &mut HashMap<ObjectId, ObjectId>,
) -> Object {
    match value {
        Object::Reference(ref_id) => {
            let mapped_id = copy_object_deep(src_doc, dest_doc, *ref_id, visited);
            Object::Reference(mapped_id)
        }
        Object::Dictionary(dict) => {
            let mut new_dict = Dictionary::new();
            for (k, v) in dict.iter() {
                new_dict.set(k.clone(), map_value_deep(src_doc, dest_doc, v, visited));
            }
            Object::Dictionary(new_dict)
        }
        Object::Array(arr) => {
            let new_arr = arr
                .iter()
                .map(|v| map_value_deep(src_doc, dest_doc, v, visited))
                .collect();
            Object::Array(new_arr)
        }
        other => other.clone(),
    }
}
