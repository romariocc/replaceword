use serde_json::Value;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use zip::{write::FileOptions, ZipArchive, ZipWriter};

use regex::Regex;

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

            replace_placeholders(&mut xml, data, None);
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

fn replace_placeholders(xml: &mut String, data: &Value, parent_key: Option<&str>) {
    if let Some(key) = parent_key {
        println!("INICIANDO parent_key: {}", key);
    } else {
        println!("INICIANDO parent_key: None");
    }

    let re =
        Regex::new(r"\$\{(?:<\/?w:[^>]+>)*((?:<\/?w:[^>]+>|\w|\.)*)(?:<\/?w:[^>]+>)*\}").unwrap();

    let mut result = xml.clone();
    println!("RESULTADO INICIAL: {}", xml);

    for caps in re.captures_iter(xml) {
        let full_match = &caps[0];
        let key_with_tags = &caps[1];
        let key_cleaned = Regex::new(r"</?w:[^>]+>")
            .unwrap()
            .replace_all(key_with_tags, "");

        println!("key_with_tags {}", key_with_tags);
        println!("key_cleaned {}", key_cleaned);

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
                    result = result.replace(full_match, key_with_tags);
                }
                Value::Number(n) => {
                    result = result.replace(full_match, &n.to_string());
                }
                Value::Array(arr) => {
                    println!("Value::Array -----------------");
                    let mut new_paragraphs = String::new();

                    // Para cada item no array, clona o parágrafo correspondente e realiza a substituição
                    for item in arr {
                        let mut cloned_paragraph = full_match.to_string();
                        replace_placeholders(&mut cloned_paragraph, item, Some(&composed_key));
                        new_paragraphs.push_str(&cloned_paragraph);
                    }

                    // Substitui o parágrafo original por todos os novos parágrafos gerados
                    result = result.replace(full_match, &new_paragraphs);
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
    println!("RESULTADO FINAL: {}", xml);
}

fn get_nested_value<'a>(data: &'a Value, key: &str) -> Option<&'a Value> {
    let keys: Vec<&str> = key.split('.').collect();
    let mut current_value = data;

    for k in keys {
        if let Ok(index) = k.parse::<usize>() {
            // Check if the key can be parsed as an integer (array index)
            if let Some(array) = current_value.as_array() {
                current_value = array.get(index)?;
                println!("get_nested_value: {}", current_value);
            } else {
                return None;
            }
        } else if let Some(value) = current_value.get(k) {
            // Otherwise treat it as an object key
            current_value = value;
        } else {
            return None;
        }
    }

    Some(current_value)
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
