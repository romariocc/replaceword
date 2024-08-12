use crate::read_paragraphs::modify_paragraphs_in_xml;
use serde_json::Value;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use zip::read::ZipArchive;
use zip::{write::FileOptions, ZipWriter};

pub fn modify_docx_paragraphs(
    input_path: &str,
    output_path: &str,
    data: &Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(input_path)?;
    let mut archive = ZipArchive::new(BufReader::new(file))?;

    // Ler o conteúdo do documento XML para um buffer de bytes e processá-lo
    let mut doc_xml = {
        let mut doc_xml_file = archive.by_name("word/document.xml")?;
        let mut doc_xml = Vec::new();
        std::io::copy(&mut doc_xml_file, &mut doc_xml)?;
        doc_xml
    };

    // Converter o buffer de bytes para uma string
    let doc_xml_str = String::from_utf8(doc_xml)?;

    // Modificar o conteúdo dos parágrafos no XML
    let modified_xml = modify_paragraphs_in_xml(&doc_xml_str, data)?;

    //println!("modified_xml: {}", modified_xml);

    let output_file = File::create(output_path)?;
    let mut zip = ZipWriter::new(BufWriter::new(output_file));

    // Iterar sobre os arquivos no arquivo ZIP, escrevendo-os no novo ZIP
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        if file.name() == "word/document.xml" {
            zip.start_file::<_, ()>("word/document.xml", FileOptions::default())?;
            zip.write_all(modified_xml.as_bytes())?;
        } else {
            zip.start_file::<_, ()>(file.name(), FileOptions::default())?;
            zip.write_all(&buffer)?;
        }
    }

    zip.finish()?;
    Ok(())
}
