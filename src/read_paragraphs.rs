use serde_json::Value;
use std::borrow::Cow;
use std::fs::File;
use std::io::BufReader;
use thiserror::Error;
use xml::attribute::Attribute;
use xml::reader::{EventReader, XmlEvent};
use xml::writer::{EmitterConfig, XmlEvent as WriterXmlEvent};
use zip::read::ZipArchive;

#[derive(Error, Debug)]
pub enum CustomError {
    #[error("IO error")]
    Io(#[from] std::io::Error),
    #[error("Zip error")]
    Zip(#[from] zip::result::ZipError),
}

pub fn read_docx_paragraphs_xml(
    file_path: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let file = File::open(file_path)?;
    let mut archive = ZipArchive::new(BufReader::new(file))?;
    let mut doc_xml = archive.by_name("word/document.xml")?;

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
                    xml_content.clear();

                    xml_content.push_str(&format!(
                        "<{}:{}",
                        name.prefix.as_ref().map(|s| s.as_str()).unwrap_or_default(),
                        name.local_name
                    ));

                    for attr in &attributes {
                        xml_content.push_str(&format!(
                            " {}:{}=\"{}\"",
                            attr.name
                                .prefix
                                .as_ref()
                                .map(|s| s.as_str())
                                .unwrap_or_default(),
                            attr.name.local_name,
                            attr.value
                        ));
                    }

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
                        name.prefix.as_ref().map(|s| s.as_str()).unwrap_or_default(),
                        name.local_name
                    ));

                    for attr in &attributes {
                        xml_content.push_str(&format!(
                            " {}:{}=\"{}\"",
                            attr.name
                                .prefix
                                .as_ref()
                                .map(|s| s.as_str())
                                .unwrap_or_default(),
                            attr.name.local_name,
                            attr.value
                        ));
                    }

                    xml_content.push('>');
                }
            }
            XmlEvent::Characters(text) => {
                if in_paragraph {
                    xml_content.push_str(&text);
                }
            }
            XmlEvent::EndElement { name } => {
                if in_paragraph {
                    xml_content.push_str(&format!(
                        "</{}:{}>",
                        name.prefix.as_ref().map(|s| s.as_str()).unwrap_or_default(),
                        name.local_name
                    ));

                    if name.local_name == "p" {
                        in_paragraph = false;
                        paragraphs.push(xml_content.clone());
                    }
                }
            }
            _ => {}
        }
    }

    Ok(paragraphs)
}

pub fn modify_paragraphs_in_xml(
    doc_xml: &str,
    data: &Value,
) -> Result<String, Box<dyn std::error::Error>> {
    let parser = EventReader::new(doc_xml.as_bytes());
    let mut output = Vec::new();
    let mut writer = EmitterConfig::new()
        .perform_indent(true)
        .create_writer(&mut output);

    let mut in_paragraph = false;
    let mut in_keyword = false;
    let mut in_repeating_block = false;
    let mut paragraph_modified = false;
    let mut paragraph_text = String::new(); // Buffer para capturar o texto do parágrafo
    let mut keyword = String::new(); // Para acumular e verificar a presença de `${`

    let mut repeating_paragraphs: Vec<XmlEvent> = Vec::new(); // Buffer para capturar parágrafos dentro do bloco de repetição
    let mut block_key = String::new(); // Para armazenar a chave do bloco de repetição

    for e in parser {
        match e? {
            XmlEvent::StartElement {
                name,
                attributes,
                namespace,
            } => {
                // Imprimir o nome do elemento antes de modificar
                //println!("StartElement - name: {}", name.local_name);

                if name.local_name == "p" {
                    in_paragraph = true;
                    paragraph_modified = false;
                    paragraph_text.clear(); // Limpar o buffer para o novo parágrafo
                    keyword.clear(); // Limpar a keyword para o novo parágrafo
                }

                let attributes: Vec<Attribute> = attributes
                    .iter()
                    .map(|attr| Attribute {
                        name: attr.name.borrow(),
                        value: attr.value.as_str(),
                    })
                    .collect();

                if in_keyword {
                    // println!("NÃO É PARAGRAFO {}", name.local_name);
                } else {
                    writer.write(WriterXmlEvent::StartElement {
                        name: name.borrow(),
                        attributes: Cow::Owned(attributes),
                        namespace: Cow::Owned(namespace),
                    })?;
                }

                /*  if name.local_name == "p" {
                    in_paragraph = true;
                    // Adicionar "Modified: " ao início do parágrafo e imprimir a modificação
                    println!("Modificando parágrafo: Adicionando 'Modified: '");
                    writer.write(WriterXmlEvent::Characters("Modified: "))?;
                } */
            }
            XmlEvent::Characters(text) => {
                if in_paragraph {
                    paragraph_text.push_str(&text); // Acumular o texto do parágrafo

                    for c in text.chars() {
                        if in_keyword {
                            keyword.push(c);

                            if keyword.ends_with("}") {
                                println!(
                                    "Sequência detectada. Saindo do modo de evitar modificação."
                                );
                                // Remover as chaves `${` e `}` e verificar se a chave existe em `data`
                                let key = &keyword[2..keyword.len() - 1]; // Extrair a chave da keyword

                                println!("keyword: {} detectada para substituição", key);

                                // Verificar chave aninhada
                                // Verificar chave aninhada
                                let mut current_value: Option<&Value> = Some(data);
                                for part in key.split('.') {
                                    println!("Acessando parte: {}", part);
                                    if let Some(Value::Object(map)) = current_value {
                                        current_value = map.get(part);
                                        println!("Valor encontrado: {:?}", current_value);
                                    } else {
                                        println!(
                                            "Parte '{}' não encontrada ou não é um objeto.",
                                            part
                                        );
                                        current_value = None;
                                        break;
                                    }
                                }

                                if let Some(value) = current_value {
                                    match value {
                                        serde_json::Value::String(value_str) => {
                                            println!(
                                                "Substituindo keyword '{}' por '{}'",
                                                key, value_str
                                            );
                                            writer.write(WriterXmlEvent::Characters(value_str))?;
                                        }
                                        serde_json::Value::Number(num) => {
                                            let value_str = num.to_string();
                                            println!(
                                                "Substituindo keyword '{}' por '{}'",
                                                key, value_str
                                            );
                                            writer.write(WriterXmlEvent::Characters(&value_str))?;
                                        }
                                        serde_json::Value::Bool(b) => {
                                            let value_str = b.to_string();
                                            println!(
                                                "Substituindo keyword '{}' por '{}'",
                                                key, value_str
                                            );
                                            writer.write(WriterXmlEvent::Characters(&value_str))?;
                                        }
                                        serde_json::Value::Null => {
                                            println!("Chave '{}' encontrada, mas é nula.", key);
                                        }
                                        _ => {
                                            println!(
                                                "Chave '{}' encontrada, mas não é uma string, número, ou booleano.",
                                                key
                                            );
                                        }
                                    }
                                } else {
                                    println!("Chave '{}' não encontrada em data.", key);
                                }
                                in_keyword = false;
                                keyword.clear();

                                if !paragraph_modified {
                                    // Adicionar "Modified: " ao início do parágrafo
                                    println!("Modificando parágrafo: Adicionando 'Modified: '");
                                    paragraph_modified = true; // Marca que o parágrafo foi modificado
                                }
                            }
                        } else {
                            if c == '$' {
                                keyword.push(c);
                            } else if keyword == "$" && c == '{' {
                                keyword.push(c);
                                in_keyword = true;
                                println!("Sequência '${{' detectada. Entrando no modo de evitar modificação.");
                            } else {
                                if !keyword.is_empty() {
                                    // Se a keyword foi iniciada mas não formou `${`, escrever `$` anterior
                                    writer.write(WriterXmlEvent::Characters("$"))?;
                                    keyword.clear();
                                }
                                // Apenas adicionar ao buffer se não estiver dentro de uma keyword
                                writer.write(WriterXmlEvent::Characters(&c.to_string()))?;
                            }
                        }
                    }
                }

                // Imprimir o conteúdo do parágrafo após modificá-lo
                /* if in_paragraph {
                    println!(
                        "Depois da modificação: ModifiedXmlEvent::Characters: {}",
                        text
                    );
                } */
            }

            XmlEvent::EndElement { name } => {
                if in_keyword {
                    //println!("NÃO É PARAGRAFO fimr {}", name.local_name);
                } else {
                    writer.write(WriterXmlEvent::EndElement {
                        name: Some(name.borrow()),
                    })?;
                }

                if in_paragraph && name.local_name == "p" {
                    in_paragraph = false;
                    println!("Parágrafo que correspondeu à regex: {}", paragraph_text);
                }
            }
            _ => {}
        }
    }

    let result = String::from_utf8(output)?;
    Ok(result)
}

fn process_keyword(
    key: &str,
    data: &Value,
    keyword: &mut String,
    paragraph_modified: &mut bool,
    writer: &mut xml::writer::EventWriter<&mut Vec<u8>>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(current_value) = get_nested_value(data, key) {
        match current_value {
            serde_json::Value::String(value_str) => {
                writer.write(WriterXmlEvent::Characters(value_str))?;
            }
            serde_json::Value::Number(num) => {
                let value_str = num.to_string();
                writer.write(WriterXmlEvent::Characters(&value_str))?;
            }
            serde_json::Value::Bool(b) => {
                let value_str = b.to_string();
                writer.write(WriterXmlEvent::Characters(&value_str))?;
            }
            serde_json::Value::Null => {
                println!("Chave '{}' encontrada, mas é nula.", key);
            }
            _ => {
                println!(
                    "Chave '{}' encontrada, mas não é uma string, número, ou booleano.",
                    key
                );
            }
        }
    } else {
        println!("Chave '{}' não encontrada em data.", key);
    }

    *paragraph_modified = true;
    keyword.clear();

    Ok(())
}

fn get_nested_value<'a>(data: &'a Value, key: &str) -> Option<&'a Value> {
    let mut current_value = Some(data);
    for part in key.split('.') {
        if let Some(Value::Object(map)) = current_value {
            current_value = map.get(part);
        } else {
            return None;
        }
    }
    current_value
}
