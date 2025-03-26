use log::error;
use std::collections::BTreeMap;
use std::io::{Cursor, Write};

use crate::proto::pdf_rendering::MetaData;
use lopdf::Dictionary as LoDictionary;
use lopdf::Object::*;
use lopdf::StringFormat::Literal;
use lopdf::{Bookmark, Document, Object, ObjectId};

pub fn merge_pdfs(documents: Vec<Document>) -> std::io::Result<Vec<u8>> {
    // Define a starting max_id (will be used as start index for object_ids)
    let mut max_id = 1;
    let mut pagenum = 1;
    // Collect all Documents Objects grouped by a map
    let mut documents_pages = BTreeMap::new();
    let mut documents_objects = BTreeMap::new();
    let mut document = Document::with_version("1.5");

    for mut doc in documents {
        let mut first = false;
        doc.renumber_objects_with(max_id);

        max_id = doc.max_id + 1;

        documents_pages.extend(
            doc.get_pages()
                .into_values()
                .map(|object_id| {
                    if !first {
                        let bookmark = Bookmark::new(
                            format!("Page_{}", pagenum),
                            [0.0, 0.0, 1.0],
                            0,
                            object_id,
                        );
                        document.add_bookmark(bookmark, None);
                        first = true;
                        pagenum += 1;
                    }

                    (object_id, doc.get_object(object_id).unwrap().to_owned())
                })
                .collect::<BTreeMap<ObjectId, Object>>(),
        );
        documents_objects.extend(doc.objects);
    }

    // Catalog and Pages are mandatory
    let mut catalog_object: Option<(ObjectId, Object)> = None;
    let mut pages_object: Option<(ObjectId, Object)> = None;

    // Process all objects except "Page" type
    for (object_id, object) in documents_objects.iter() {
        // We have to ignore "Page" (as are processed later), "Outlines" and "Outline" objects
        // All other objects should be collected and inserted into the main Document
        match object.type_name().unwrap_or("".as_bytes()) {
            b"Catalog" => {
                // Collect a first "Catalog" object and use it for the future "Pages"
                catalog_object = Some((
                    if let Some((id, _)) = catalog_object {
                        id
                    } else {
                        *object_id
                    },
                    object.clone(),
                ));
            }
            b"Pages" => {
                // Collect and update a first "Pages" object and use it for the future "Catalog"
                // We have also to merge all dictionaries of the old and the new "Pages" object
                if let Ok(dictionary) = object.as_dict() {
                    let mut dictionary = dictionary.clone();
                    if let Some((_, ref object)) = pages_object {
                        if let Ok(old_dictionary) = object.as_dict() {
                            dictionary.extend(old_dictionary);
                        }
                    }

                    pages_object = Some((
                        if let Some((id, _)) = pages_object {
                            id
                        } else {
                            *object_id
                        },
                        Dictionary(dictionary),
                    ));
                }
            }
            b"Page" => {}     // Ignored, processed later and separately
            b"Outlines" => {} // Ignored, not supported yet
            b"Outline" => {}  // Ignored, not supported yet
            _ => {
                document.objects.insert(*object_id, object.clone());
            }
        }
    }

    // If no "Pages" object found abort
    if pages_object.is_none() {
        error!("Pages root not found.");

        return Ok(vec![]);
    }

    // Iterate over all "Page" objects and collect into the parent "Pages" created before
    for (object_id, object) in documents_pages.iter() {
        if let Ok(dictionary) = object.as_dict() {
            let mut dictionary = dictionary.clone();
            dictionary.set("Parent", pages_object.as_ref().unwrap().0);

            document.objects.insert(*object_id, Dictionary(dictionary));
        }
    }

    // If no "Catalog" found abort
    if catalog_object.is_none() {
        error!("Catalog root not found.");

        return Ok(vec![]);
    }

    let catalog_object = catalog_object.unwrap();
    let pages_object = pages_object.unwrap();

    // Build a new "Pages" with updated fields
    if let Ok(dictionary) = pages_object.1.as_dict() {
        let mut dictionary = dictionary.clone();

        // Set new pages count
        dictionary.set("Count", documents_pages.len() as u32);

        // Set new "Kids" list (collected from documents pages) for "Pages"
        dictionary.set(
            "Kids",
            documents_pages
                .into_keys()
                .map(Reference)
                .collect::<Vec<_>>(),
        );

        document
            .objects
            .insert(pages_object.0, Dictionary(dictionary));
    }

    // Build a new "Catalog" with updated fields
    if let Ok(dictionary) = catalog_object.1.as_dict() {
        let mut dictionary = dictionary.clone();
        dictionary.set("Pages", pages_object.0);
        dictionary.remove(b"Outlines"); // Outlines not supported in merged PDFs

        document
            .objects
            .insert(catalog_object.0, Dictionary(dictionary));
    }

    document.trailer.set("Root", catalog_object.0);

    // Update the max internal ID as wasn't updated before due to direct objects insertion
    document.max_id = document.objects.len() as u32;

    // Reorder all new Document objects
    document.renumber_objects();

    //Set any Bookmarks to the First child if they are not set to a page
    document.adjust_zero_pages();

    //Set all bookmarks to the PDF Object tree then set the Outlines to the Bookmark content map.
    if let Some(n) = document.build_outline() {
        if let Ok(Dictionary(ref mut dict)) = document.get_object_mut(catalog_object.0) {
            dict.set("Outlines", Reference(n));
        }
    }

    document.compress();

    let out_buf = Vec::new();
    let mut memory_cursor = Cursor::new(out_buf.clone());

    document
        .save_to(&mut memory_cursor)
        .expect("failed saving pdf");

    memory_cursor.flush().expect("failed flushing");

    Ok(memory_cursor.get_ref().to_vec())
}

pub fn add_pdf_metadata(file: Vec<u8>, meta: Option<MetaData>) -> std::io::Result<Vec<u8>> {
    match meta {
        None => Ok(file),
        Some(m) => {
            let mut document = Document::load_mem(&file).expect("failed parsing pdf");

            let mut meta_list = Vec::new();

            if m.title.is_some() {
                meta_list.push(("Title", String(m.title.unwrap().into(), Literal)));
            }

            if m.creator.is_some() {
                meta_list.push(("Creator", String(m.creator.unwrap().into(), Literal)));
            }

            if m.producer.is_some() {
                meta_list.push(("Producer", String(m.producer.unwrap().into(), Literal)));
            }

            document.set_object((1, 0), LoDictionary::from_iter(meta_list.clone()));

            document.compress();

            let out_buf = Vec::new();
            let mut memory_cursor = Cursor::new(out_buf.clone());

            document
                .save_to(&mut memory_cursor)
                .expect("failed saving PDF");

            memory_cursor.flush().expect("failed flushing");

            Ok(memory_cursor.get_ref().to_vec())
        }
    }
}
