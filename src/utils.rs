use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use zip::ZipArchive;

/// Auxiliary function to extract the `document.xml` content from a DOCX file.
/// Returns the content as a `String`.
pub fn extract_document_xml<P: AsRef<Path> + std::fmt::Debug>(
    docx_path: P,
) -> Result<String, Box<dyn std::error::Error>> {
    // Open the DOCX file (which is a ZIP archive)
    println!("endereço:{:?}", docx_path);
    let file = File::open(docx_path)?;
    let mut archive = ZipArchive::new(BufReader::new(file))?;

    // Extract the content of "word/document.xml" and return it as a String
    let mut doc_xml_file = archive.by_name("word/document.xml")?;
    let mut doc_xml = String::new();
    doc_xml_file.read_to_string(&mut doc_xml)?;

    Ok(doc_xml)
}

// Example test using the extract_document_xml function
#[test]
fn test_extract_document_xml() {
    let docx_path = "tests/modelo.dotx"; // path to your test DOCX file
    let doc_xml = extract_document_xml(docx_path).unwrap();

    println!("Segue docuemnto:{:?}", doc_xml);
    // Perform assertions on `doc_xml` content
    assert!(doc_xml.contains("document"));
    // Add more assertions as needed
}
