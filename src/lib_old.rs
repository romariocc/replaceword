use serde_json::Value;
use std::io::{BufWriter, Read, Write};
use std::path::Path;
use zip::{write::FileOptions, ZipArchive, ZipWriter};

use regex::Regex;

use std::fs::File;
use std::io::BufReader;

use thiserror::Error;

use xml::reader::{EventReader, XmlEvent};

use std::borrow::Cow;
use xml::writer::{EmitterConfig, XmlEvent as WriterXmlEvent};

#[derive(Error, Debug)]
pub enum CustomError {
    #[error("IO error")]
    Io(#[from] std::io::Error),
    #[error("Docx error")]
    Docx(#[from] docx_rs::DocxError),
    #[error("Zip error")]
    Zip(#[from] zip::result::ZipError),
}

/// Função que substitui placeholders em um arquivo Word (.dotx) com dados fornecidos.
///
/// # Exemplo
///
/// ```
/// use serde_json::json;
/// use std::fs;
/// use wordreplacelib::replace;
///
/// let data = json!({
///     "nome": "João",
///     "profissao": "Engenheiro",
///     "eventos": [
///         {
///             "nome": "Aniversário",
///             "data": "2024-01-01",
///             "descricao": "Festa de aniversário"
///         },
///         {
///             "nome": "Casamento",
///             "data": "2024-06-15",
///             "descricao": "Cerimônia de casamento"
///         }
///     ]
/// });
///
/// let input_path = "tests/modelo.dotx";
/// let output_path = "tests/output";
/// let output_filename = "doctest.doc";
///
/// let result = replace(input_path, &data, output_path, output_filename);
/// assert!(result.is_ok());
///
/// let output_file_path = format!("{}/{}", output_path, output_filename);
/// let metadata = fs::metadata(&output_file_path);
/// assert!(metadata.is_ok());
/// ```
pub fn replace(
    input_path: &str,
    data: &Value,
    output_path: &str,
    output_filename: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if !std::path::Path::new(output_path).exists() {
        std::fs::create_dir_all(output_path)?;
    }

    //let _ = read_docx_paragraphs_xml();

    let output_path = "output.docx";

    match modify_docx_paragraphs(input_path, output_path) {
        Ok(_) => println!("Novo documento criado com sucesso!"),
        Err(e) => eprintln!("Erro ao criar novo documento: {}", e),
    }

    let filename_without_extension = Path::new(output_filename)
        .file_stem()
        .unwrap_or_else(|| output_filename.as_ref())
        .to_str()
        .unwrap_or(output_filename);

    let output_file_path = format!("{}/{}.doc", output_path, filename_without_extension);

    //let output_file_path = format!("{}/{}", output_path, output_filename);

    let file = File::open(input_path)?;
    let mut zip = ZipArchive::new(file)?;

    let output_file = File::create(&output_file_path)?;
    let mut zip_writer = ZipWriter::new(output_file);

    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        let name = file.name().to_string();

        if name == "word/document.xml" {
            let mut xml = String::new();
            file.read_to_string(&mut xml)?;

            //process_array_blocks(&mut xml, data);

            //replace_placeholders(&mut xml, data, None);
            //replace_placeholders(&mut xml_content, &data, None);

            zip_writer.start_file::<_, ()>("word/document.xml", FileOptions::default())?;
            zip_writer.write_all(xml.as_bytes())?;
        } else {
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            zip_writer.start_file::<_, ()>(&name, FileOptions::default())?;
            zip_writer.write_all(&buffer)?;
        }
    }

    zip_writer.finish()?;
    println!("Documento gerado: {}", output_file_path);
    Ok(())
}

fn type_of(value: &Value) -> &str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn process_array_blocks(xml: &mut String, data: &Value) {
    println!("xml: {}", xml);

    println!("xmsl: {}", xml);

    let start_re = Regex::new(r"<\/[^>]+>\s*\$\{#.*?\}.*?<\/w:p>").unwrap(); // Captura qualquer bloco de array mesmo com tags internas

    while let Some(start_match) = start_re.find(xml) {
        let block_start_pos = start_match.start();
        let block_end_pos = start_match.end();
        let block_start_content = &xml[block_start_pos..block_end_pos];

        println!("block_start_content: {}", block_start_content);

        // Extrai o nome do bloco de array, removendo as tags internas
        let block_name_with_tags = block_start_content
            .trim_start_matches("${#")
            .trim_end_matches("}");

        let block_name_clean = Regex::new(r"</?w:[^>]+>")
            .unwrap()
            .replace_all(block_name_with_tags, "")
            .to_string();

        println!("block_name_with_tags: {}", block_name_with_tags);

        // Cria a expressão regular para o fim do bloco usando o nome extraído
        let mut pattern = String::new();
        for c in block_name_clean.chars() {
            pattern.push(c);
            pattern.push_str(r"(?:</?w:[^>]+>)*");
        }

        let end_re = Regex::new(&format!(
            r"\$\{{/(?:</?w:[^>]+>)*{}(?:</?w:[^>]+>)*\}}",
            pattern
        ))
        .unwrap();

        if let Some(end_match) = end_re.find(&xml[block_end_pos..]) {
            let content_start_pos = block_end_pos;
            let content_end_pos = block_end_pos + end_match.start();
            let block_content = &xml[content_start_pos..content_end_pos];

            // Substituição do bloco para cada elemento do array
            if let Some(Value::Array(arr)) = get_nested_value_array(data, &block_name_clean) {
                let mut new_content = String::new();
                for item in arr {
                    let mut cloned_content = block_content.to_string();
                    replace_placeholders(&mut cloned_content, item, Some(&block_name_clean));
                    new_content.push_str(&cloned_content);
                }

                println!("new_content: {}", new_content);

                /*  // Substitui o conteúdo do bloco completo com as novas entradas para cada item do array
                xml.replace_range(
                    block_start_pos..content_end_pos + end_match.end(),
                    &new_content,
                ); */
            }
        }

        // Remover o bloco do XML
        xml.replace_range(block_start_pos..block_end_pos, "");
    }
}

fn replace_placeholders(xml: &mut String, data: &Value, parent_key: Option<&str>) {
    if let Some(key) = parent_key {
        println!("INICIANDO parent_key: {}", key);
    } else {
        println!("INICIANDO parent_key: None");
    }

    let re = Regex::new(r"\$\{(?:<\/?w:[^>]+>)*((?:<\/?w:[^>]+>|\w|\.)*)(?:<\/?w:[^>]+>)*\}|\$\{#(\w+)\}|\$\{\/(\w+)\}")
        .unwrap();

    let mut result = xml.clone();
    //println!("RESULTADO INICIAL: {}", xml);

    //let block_stack: Vec<(String, String)> = Vec::new();
    let mut in_block = false;
    let mut block_content = String::new();
    let mut block_name = String::new();

    for caps in re.captures_iter(xml) {
        let full_match = &caps[0];

        if let Some(array_start) = caps.get(2) {
            // Início de um bloco array
            block_name = array_start.as_str().to_string();
            in_block = true;
            block_content.clear();
            continue;
        }

        if let Some(array_end) = caps.get(3) {
            // Fim de um bloco array
            if in_block && array_end.as_str() == block_name {
                // Substituição do bloco para cada elemento do array
                if let Some(Value::Array(arr)) = get_nested_value(data, &block_name) {
                    let mut new_content = String::new();
                    for item in arr {
                        let mut cloned_content = block_content.clone();
                        replace_placeholders(&mut cloned_content, item, Some(&block_name));
                        new_content.push_str(&cloned_content);
                    }
                    result =
                        result.replace(&format!("${{{}{}{}", "#", block_name, "}"), &new_content);
                    result = result.replace(&format!("${{{}{}{}", "/", block_name, "}"), "");
                }
                in_block = false;
            }
            continue;
        }

        if in_block {
            block_content.push_str(full_match);
            continue;
        }

        let key_with_tags = &caps[1];
        let key_cleaned = Regex::new(r"</?w:[^>]+>")
            .unwrap()
            .replace_all(key_with_tags, "");

        let composed_key = if let Some(parent) = parent_key {
            format!("{}.{}", parent, key_cleaned)
        } else {
            key_cleaned.to_string()
        };

        if let Some(value) = get_nested_value(data, &composed_key) {
            println!("Tipo do valor: {} para {}", type_of(value), &composed_key);
            match value {
                Value::String(s) => {
                    println!("Substituindo placeholder {} por {}", composed_key, s);
                    result = result.replace(full_match, s);
                }
                Value::Number(n) => {
                    result = result.replace(full_match, &n.to_string());
                }
                Value::Object(_) => {
                    println!("entrou {}", key_cleaned);
                    replace_placeholders(&mut result, value, Some(&composed_key));
                }
                _ => {}
            }
        } else {
            println!("Placeholder {} não encontrado nos dados", composed_key);
        }
    }

    *xml = result;
    // println!("RESULTADO FINAL: {}", xml);
}

fn get_nested_value_array<'a>(data: &'a Value, key: &str) -> Option<&'a Value> {
    let keys: Vec<&str> = key.split('.').collect();
    let mut current_value = data;

    for k in keys {
        match current_value {
            Value::Object(map) => {
                if let Some(value) = map.get(k) {
                    current_value = value;
                } else {
                    return None;
                }
            }
            Value::Array(array) => {
                if let Ok(index) = k.parse::<usize>() {
                    if let Some(value) = array.get(index) {
                        current_value = value;
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }

    Some(current_value)
}

fn get_nested_value<'a>(data: &'a Value, key: &str) -> Option<&'a Value> {
    let keys: Vec<&str> = key.split('.').collect();
    let mut current_value = data;

    // println!("current_value: {} - key {} ", current_value, key);

    for k in keys {
        if let Ok(index) = k.parse::<usize>() {
            if let Some(array) = current_value.as_array() {
                current_value = array.get(index)?;
                println!("get_nested_value: {}", current_value);
            } else {
                return None;
            }
        } else if let Some(value) = current_value.get(k) {
            current_value = value;
        } else {
            return None;
        }
    }

    Some(current_value)
}

pub fn create_and_iterate_paragraphs() -> Result<(), DocxError> {
    let path = std::path::Path::new("./output.docx");
    let file = std::fs::File::create(path).unwrap();

    let docx = Docx::new()
        .add_paragraph(Paragraph::new().add_run(Run::new().add_text("Hello, World!")))
        .add_paragraph(Paragraph::new().add_run(Run::new().add_text("This is another paragraph.")));

    docx.build().pack(file)?;

    Ok(())
}

fn create_docx_from_paragraphs(
    paragraphs: Vec<String>,
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut docx = Docx::new();

    for paragraph_xml in paragraphs {
        // Adiciona cada parágrafo ao novo documento
        docx = docx.add_paragraph(Paragraph::new().add_run(Run::new().add_text(paragraph_xml)));
    }

    let mut file = File::create(output_path)?;
    docx.build().pack(file)?;

    Ok(())
}

fn read_docx_paragraphs_xml(file_path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Abrir o arquivo .docx como um arquivo ZIP
    let file = File::open(file_path)?;
    let mut archive = ZipArchive::new(BufReader::new(file))?;

    // Abrir o arquivo XML do documento principal
    let mut doc_xml = archive.by_name("word/document.xml")?;

    // Ler e processar o XML
    let parser = EventReader::new(&mut doc_xml);
    let mut paragraphs = Vec::new();
    let mut xml_content = String::new();
    let mut in_paragraph = false;

    for e in parser {
        match e? {
            XmlEvent::StartElement {
                name,
                attributes,
                namespace,
            } => {
                if name.local_name == "p" {
                    in_paragraph = true;
                    xml_content.clear(); // Limpar o conteúdo anterior para armazenar o novo parágrafo

                    // Adiciona a declaração do namespace ao início do parágrafo
                    xml_content.push_str(&format!(
                        "<{}:{}",
                        name.prefix.unwrap_or_default(),
                        name.local_name
                    ));

                    for attr in attributes {
                        xml_content.push_str(&format!(
                            " {}:{}=\"{}\"",
                            attr.name.prefix.unwrap_or_default(),
                            attr.name.local_name,
                            attr.value
                        ));
                    }

                    // Adicionar namespaces ao primeiro elemento
                    for (prefix, uri) in &namespace {
                        if prefix.is_empty() {
                            xml_content.push_str(&format!(" xmlns=\"{}\"", uri));
                        } else {
                            xml_content.push_str(&format!(" xmlns:{}=\"{}\"", prefix, uri));
                        }
                    }

                    xml_content.push('>');
                } else if in_paragraph {
                    xml_content.push_str(&format!(
                        "<{}:{}",
                        name.prefix.unwrap_or_default(),
                        name.local_name
                    ));

                    for attr in attributes {
                        xml_content.push_str(&format!(
                            " {}:{}=\"{}\"",
                            attr.name.prefix.unwrap_or_default(),
                            attr.name.local_name,
                            attr.value
                        ));
                    }

                    xml_content.push('>');
                }
            }
            XmlEvent::Characters(text) => {
                if in_paragraph {
                    // Adiciona o texto contido dentro da tag ao XML
                    xml_content.push_str(&text);
                }
            }
            XmlEvent::EndElement { name } => {
                if in_paragraph {
                    xml_content.push_str(&format!(
                        "</{}:{}>",
                        name.prefix.unwrap_or_default(),
                        name.local_name
                    ));

                    if name.local_name == "p" {
                        in_paragraph = false;
                        paragraphs.push(xml_content.clone()); // Armazena o parágrafo no vetor
                    }
                }
            }
            _ => {}
        }
    }

    Ok(paragraphs)
}

pub fn modify_paragraphs_in_xml(doc_xml: &str) -> Result<String, Box<dyn std::error::Error>> {
    let parser = EventReader::new(doc_xml.as_bytes());
    let mut output = Vec::new();
    let mut writer = EmitterConfig::new()
        .perform_indent(true)
        .create_writer(&mut output);

    let mut in_paragraph = false;

    for e in parser {
        match e? {
            XmlEvent::StartElement {
                name,
                attributes,
                namespace,
            } => {
                if name.local_name == "p" {
                    in_paragraph = true;
                }

                // Convertendo para os tipos esperados
                let name = Name::from(name);
                let attributes = Cow::Owned(attributes);
                let namespace = Cow::Owned(namespace);

                writer.write(WriterXmlEvent::StartElement {
                    name,
                    attributes,
                    namespace,
                })?;

                // Modificar o conteúdo do parágrafo aqui
                if in_paragraph && name.local_name == "p" {
                    // Exemplo: Adiciona um texto ao início de cada parágrafo
                    writer.write(WriterXmlEvent::Characters("Modified: "))?;
                }
            }
            XmlEvent::Characters(text) => {
                writer.write(WriterXmlEvent::Characters(&text))?;
            }
            XmlEvent::EndElement { name } => {
                writer.write(WriterXmlEvent::EndElement {
                    name: Some(Name::from(name)),
                })?;

                if in_paragraph && name.local_name == "p" {
                    in_paragraph = false;
                }
            }
            other => {
                writer.write(other)?;
            }
        }
    }

    let result = String::from_utf8(output)?;
    Ok(result)
}

fn modify_docx_paragraphs(
    input_path: &str,
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Abrir o arquivo .docx original como um arquivo ZIP
    let file = File::open(input_path)?;
    let mut archive = ZipArchive::new(BufReader::new(file))?;

    // Extrair o XML do documento original
    let mut doc_xml = String::new();
    archive
        .by_name("word/document.xml")?
        .read_to_string(&mut doc_xml)?;

    // Modificar os parágrafos no XML
    let modified_xml = modify_paragraphs_in_xml(&doc_xml)?;

    // Recriar o arquivo .docx com o XML modificado
    let output_file = File::create(output_path)?;
    let mut zip = ZipWriter::new(BufWriter::new(output_file));

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        if file.name() == "word/document.xml" {
            // Escrever o XML modificado para o documento
            zip.start_file::<_, ()>("word/document.xml", FileOptions::default())?;
            zip.write_all(modified_xml.as_bytes())?;
        } else {
            // Copiar os outros arquivos como estão
            zip.start_file::<_, ()>(file.name(), FileOptions::default())?;
            zip.write_all(&buffer)?;
        }
    }

    zip.finish()?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;

    #[test]
    fn test_replace() {
        println!("test_replace");
        let data = json!({
            "nome": "João Carlos",
            "profissao": "Engenheiro",
            "casa": {
                "quartos": 3,
                "banheiros": 2
            },
            "evanto": [
                {
                    "nome": "Aniversário",
                    "data": "2024-01-01",
                    "descricao": "Festa de aniversário"
                },
                {
                    "nome": "Casamento",
                    "data": "2024-06-15",
                    "descricao": "Cerimônia de casamento"
                }
            ]
        });

        let input_path = "tests/modelo.dotx";
        let output_path = "tests/output";
        let output_filename = "documento_final.doc";

        let result = replace(input_path, &data, output_path, output_filename);

        println!("Resultado do replace: {:?}", result);
        assert!(result.is_ok());

        let output_file_path = format!("{}/{}", output_path, output_filename);
        let metadata = fs::metadata(&output_file_path);
        println!("Metadata do arquivo gerado: {:?}", metadata);
        assert!(metadata.is_ok());
    }
}
