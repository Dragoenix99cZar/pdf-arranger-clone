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

    // Keep track of visited object IDs per source document to avoid duplicate copying across pages
    let mut visited_per_doc: Vec<HashMap<ObjectId, ObjectId>> =
        vec![HashMap::new(); source_docs.len()];

    // Maps to translate source page targets into destination page ObjectIds for bookmarks
    let mut src_page_to_dest_page: HashMap<(usize, ObjectId), ObjectId> = HashMap::new();
    let mut src_page_num_to_dest_page: HashMap<(usize, u32), ObjectId> = HashMap::new();

    for page_item in &project.pages {
        let src_doc = &source_docs[page_item.source_document];
        let visited = &mut visited_per_doc[page_item.source_document];
        let pages_map = src_doc.get_pages();

        if let Some(&src_page_id) = pages_map.get(&page_item.source_page) {
            let new_page_id = copy_page_deep(src_doc, &mut dest_doc, src_page_id, visited);
            cloned_page_ids.push(new_page_id);

            // Record destination page for outline linking (first occurrence)
            src_page_to_dest_page
                .entry((page_item.source_document, src_page_id))
                .or_insert(new_page_id);
            src_page_num_to_dest_page
                .entry((page_item.source_document, page_item.source_page))
                .or_insert(new_page_id);
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

    // Extract, remap, and reconstruct Bookmarks / Outlines across source documents
    let mut all_outlines = Vec::new();
    for (doc_idx, src_doc) in source_docs.iter().enumerate() {
        if let Some(catalog) = get_catalog(src_doc) {
            if let Ok(outlines_ref) = catalog.get(b"Outlines") {
                let outlines = deref(src_doc, outlines_ref);
                if let Object::Dictionary(outlines_dict) = outlines {
                    if let Ok(first_obj) = outlines_dict.get(b"First") {
                        let mut curr_obj = Some(first_obj);
                        let mut visited_items = std::collections::HashSet::new();

                        while let Some(obj) = curr_obj {
                            if let Object::Reference(item_id) = obj {
                                if let Some(item_node) = parse_outline_item(
                                    src_doc,
                                    doc_idx,
                                    *item_id,
                                    &src_page_to_dest_page,
                                    &src_page_num_to_dest_page,
                                    &mut visited_items,
                                ) {
                                    all_outlines.push(item_node);
                                }
                                if let Ok(Object::Dictionary(dict)) = src_doc.get_object(*item_id) {
                                    curr_obj = dict.get(b"Next").ok();
                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    if !all_outlines.is_empty() {
        let outlines_id = dest_doc.new_object_id();
        let (first_id, last_id, count) =
            write_outline_items(&mut dest_doc, &all_outlines, outlines_id);

        let mut outlines_dict = Dictionary::new();
        outlines_dict.set("Type", Object::Name(b"Outlines".to_vec()));
        if let Some(first) = first_id {
            outlines_dict.set("First", Object::Reference(first));
        }
        if let Some(last) = last_id {
            outlines_dict.set("Last", Object::Reference(last));
        }
        outlines_dict.set("Count", Object::Integer(count));

        dest_doc
            .objects
            .insert(outlines_id, Object::Dictionary(outlines_dict));
        catalog_dict.set("Outlines", Object::Reference(outlines_id));
        catalog_dict.set("PageMode", Object::Name(b"UseOutlines".to_vec()));
    }

    dest_doc
        .objects
        .insert(catalog_id, Object::Dictionary(catalog_dict));

    dest_doc.trailer.set("Root", Object::Reference(catalog_id));
    dest_doc.max_id = dest_doc.objects.len() as u32;

    // Prune unreferenced objects and compress streams
    dest_doc.prune_objects();
    dest_doc.compress();

    // Save the generated document to the target path
    dest_doc.save(output_path)?;

    Ok(())
}

/// Represents a parsed bookmark node with destinations translated to the new document.
struct ParsedOutlineItem {
    title: Vec<u8>,
    dest: Option<Vec<Object>>,
    action: Option<Object>,
    color: Option<Object>,
    flags: Option<i64>,
    is_closed: bool,
    children: Vec<ParsedOutlineItem>,
}

fn get_catalog(doc: &Document) -> Option<&Dictionary> {
    let root = doc.trailer.get(b"Root").ok()?;
    match root {
        Object::Reference(id) => match doc.get_object(*id).ok()? {
            Object::Dictionary(dict) => Some(dict),
            _ => None,
        },
        Object::Dictionary(dict) => Some(dict),
        _ => None,
    }
}

fn deref<'a>(doc: &'a Document, obj: &'a Object) -> &'a Object {
    match obj {
        Object::Reference(id) => doc.get_object(*id).unwrap_or(obj),
        other => other,
    }
}

fn parse_outline_item(
    src_doc: &Document,
    doc_idx: usize,
    item_id: ObjectId,
    page_map: &HashMap<(usize, ObjectId), ObjectId>,
    page_num_map: &HashMap<(usize, u32), ObjectId>,
    visited_items: &mut std::collections::HashSet<ObjectId>,
) -> Option<ParsedOutlineItem> {
    if !visited_items.insert(item_id) {
        return None;
    }

    let item_dict = match src_doc.get_object(item_id).ok()? {
        Object::Dictionary(d) => d,
        _ => return None,
    };

    let title = match item_dict.get(b"Title").ok() {
        Some(Object::String(bytes, _)) => bytes.clone(),
        Some(Object::Name(name)) => name.clone(),
        _ => b"Untitled".to_vec(),
    };

    let mut dest_arr = None;
    let mut action_obj = None;

    if let Ok(dest) = item_dict.get(b"Dest") {
        dest_arr = resolve_dest_array(src_doc, dest);
    } else if let Ok(action) = item_dict.get(b"A") {
        let action_deref = deref(src_doc, action);
        if let Object::Dictionary(a_dict) = action_deref {
            if let Ok(Object::Name(s)) = a_dict.get(b"S") {
                if s == b"GoTo" {
                    if let Ok(d) = a_dict.get(b"D") {
                        dest_arr = resolve_dest_array(src_doc, d);
                    }
                } else {
                    action_obj = Some(action_deref.clone());
                }
            }
        }
    }

    let mut remapped_dest = None;
    if let Some(mut arr) = dest_arr {
        if !arr.is_empty() {
            let target_page_id = match &arr[0] {
                Object::Reference(ref_id) => page_map.get(&(doc_idx, *ref_id)).copied(),
                Object::Integer(idx) => {
                    let page_num = (*idx as u32) + 1;
                    page_num_map.get(&(doc_idx, page_num)).copied()
                }
                _ => None,
            };

            if let Some(new_page_id) = target_page_id {
                arr[0] = Object::Reference(new_page_id);
                remapped_dest = Some(arr);
            }
        }
    }

    // Parse children items
    let mut children = Vec::new();
    if let Ok(first_obj) = item_dict.get(b"First") {
        let mut curr_obj = Some(first_obj);
        while let Some(obj) = curr_obj {
            if let Object::Reference(child_id) = obj {
                if let Some(child_node) = parse_outline_item(
                    src_doc,
                    doc_idx,
                    *child_id,
                    page_map,
                    page_num_map,
                    visited_items,
                ) {
                    children.push(child_node);
                }
                if let Ok(Object::Dictionary(dict)) = src_doc.get_object(*child_id) {
                    curr_obj = dict.get(b"Next").ok();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    // If target page was deleted and no children survive, prune this outline item
    if remapped_dest.is_none() && children.is_empty() && action_obj.is_none() {
        return None;
    }

    let color = item_dict.get(b"C").ok().map(|c| deref(src_doc, c).clone());
    let flags = match item_dict.get(b"F").ok() {
        Some(Object::Integer(f)) => Some(*f),
        _ => None,
    };
    let is_closed = match item_dict.get(b"Count").ok() {
        Some(Object::Integer(c)) => *c < 0,
        _ => false,
    };

    Some(ParsedOutlineItem {
        title,
        dest: remapped_dest,
        action: action_obj,
        color,
        flags,
        is_closed,
        children,
    })
}

fn resolve_dest_array(src_doc: &Document, dest_obj: &Object) -> Option<Vec<Object>> {
    let dest = deref(src_doc, dest_obj);
    match dest {
        Object::Array(arr) => Some(arr.clone()),
        Object::Name(name) => resolve_named_dest(src_doc, name),
        Object::String(name, _) => resolve_named_dest(src_doc, name),
        _ => None,
    }
}

fn resolve_named_dest(src_doc: &Document, name: &[u8]) -> Option<Vec<Object>> {
    let catalog = get_catalog(src_doc)?;

    // 1. Check Catalog -> /Dests
    if let Ok(dests_obj) = catalog.get(b"Dests") {
        let dests = deref(src_doc, dests_obj);
        if let Object::Dictionary(dict) = dests {
            if let Ok(val) = dict.get(name) {
                let val = deref(src_doc, val);
                if let Object::Array(arr) = val {
                    return Some(arr.clone());
                } else if let Object::Dictionary(d_dict) = val {
                    if let Ok(d_val) = d_dict.get(b"D") {
                        let d_val = deref(src_doc, d_val);
                        if let Object::Array(arr) = d_val {
                            return Some(arr.clone());
                        }
                    }
                }
            }
        }
    }

    // 2. Check Catalog -> /Names -> /Dests (Name Tree)
    if let Ok(names_obj) = catalog.get(b"Names") {
        let names = deref(src_doc, names_obj);
        if let Object::Dictionary(names_dict) = names {
            if let Ok(dests_tree_obj) = names_dict.get(b"Dests") {
                return search_name_tree(src_doc, dests_tree_obj, name);
            }
        }
    }

    None
}

fn search_name_tree(src_doc: &Document, node_obj: &Object, target: &[u8]) -> Option<Vec<Object>> {
    let node = deref(src_doc, node_obj);
    if let Object::Dictionary(dict) = node {
        if let Ok(Object::Array(names_arr)) = dict.get(b"Names") {
            for chunk in names_arr.chunks(2) {
                if chunk.len() == 2 {
                    let key_match = match &chunk[0] {
                        Object::String(s, _) => s.as_slice() == target,
                        Object::Name(n) => n.as_slice() == target,
                        _ => false,
                    };
                    if key_match {
                        let val = deref(src_doc, &chunk[1]);
                        if let Object::Array(arr) = val {
                            return Some(arr.clone());
                        } else if let Object::Dictionary(d_dict) = val {
                            if let Ok(d_val) = d_dict.get(b"D") {
                                let d_val = deref(src_doc, d_val);
                                if let Object::Array(arr) = d_val {
                                    return Some(arr.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
        if let Ok(Object::Array(kids_arr)) = dict.get(b"Kids") {
            for kid in kids_arr {
                if let Some(res) = search_name_tree(src_doc, kid, target) {
                    return Some(res);
                }
            }
        }
    }
    None
}

fn write_outline_items(
    dest_doc: &mut Document,
    items: &[ParsedOutlineItem],
    parent_id: ObjectId,
) -> (Option<ObjectId>, Option<ObjectId>, i64) {
    if items.is_empty() {
        return (None, None, 0);
    }

    let item_ids: Vec<ObjectId> = items.iter().map(|_| dest_doc.new_object_id()).collect();
    let mut total_count: i64 = 0;

    for (i, item) in items.iter().enumerate() {
        let item_id = item_ids[i];
        let mut dict = Dictionary::new();

        dict.set(
            "Title",
            Object::String(item.title.clone(), lopdf::StringFormat::Literal),
        );
        dict.set("Parent", Object::Reference(parent_id));

        if i > 0 {
            dict.set("Prev", Object::Reference(item_ids[i - 1]));
        }
        if i + 1 < item_ids.len() {
            dict.set("Next", Object::Reference(item_ids[i + 1]));
        }

        if let Some(dest) = &item.dest {
            dict.set("Dest", Object::Array(dest.clone()));
        } else if let Some(action) = &item.action {
            dict.set("A", action.clone());
        }

        if let Some(color) = &item.color {
            dict.set("C", color.clone());
        }
        if let Some(flags) = item.flags {
            dict.set("F", Object::Integer(flags));
        }

        if !item.children.is_empty() {
            let (child_first, child_last, child_count) =
                write_outline_items(dest_doc, &item.children, item_id);

            if let Some(first_id) = child_first {
                dict.set("First", Object::Reference(first_id));
            }
            if let Some(last_id) = child_last {
                dict.set("Last", Object::Reference(last_id));
            }

            if child_count > 0 {
                let display_count = if item.is_closed { -child_count } else { child_count };
                dict.set("Count", Object::Integer(display_count));
            }

            total_count += 1 + child_count;
        } else {
            total_count += 1;
        }

        dest_doc.objects.insert(item_id, Object::Dictionary(dict));
    }

    let first = item_ids.first().copied();
    let last = item_ids.last().copied();
    (first, last, total_count)
}

/// Recursively copy a page object. A fresh page dictionary object ID is always allocated
/// in `dest_doc` so each project page is independent in the page tree, but all its child
/// resources (fonts, images, contents) are mapped through `visited` to avoid duplication.
fn copy_page_deep(
    src_doc: &Document,
    dest_doc: &mut Document,
    src_page_id: ObjectId,
    visited: &mut HashMap<ObjectId, ObjectId>,
) -> ObjectId {
    let obj = match src_doc.get_object(src_page_id) {
        Ok(o) => o,
        Err(_) => return src_page_id,
    };

    let new_id = dest_doc.new_object_id();

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
        _ => obj.clone(),
    };

    dest_doc.objects.insert(new_id, cloned_obj);
    new_id
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
