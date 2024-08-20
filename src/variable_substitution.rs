use crate::translation::{format_date_with_locale, LOCALE, TRANSLATIONS};
use chrono::{Datelike, NaiveDate};
use serde_json::Value;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::io::Cursor;
use sys_locale::get_locale;
use xml::attribute::OwnedAttribute;
use xml::name::OwnedName;
use xml::reader::{EventReader, XmlEvent};
use xml::writer::{EmitterConfig, EventWriter, XmlEvent as WriterXmlEvent};

fn process_word_xml_with_substitutions(
    doc_xml: &str,
    data: &Value,
) -> Result<String, Box<dyn std::error::Error>> {
    let cursor = Cursor::new(doc_xml.as_bytes());
    let mut parser = EventReader::new(cursor);
    let mut path_stack: VecDeque<String> = VecDeque::new(); // To track the XML element path

    let mut main_output = Vec::new();
    let mut temp_output = Vec::new();
    let mut writer = EmitterConfig::new()
        .perform_indent(true)
        .create_writer(&mut temp_output);

    let mut accumulated_text = String::new();
    let mut in_keyword = false; // Flag indicating whether we're inside a keyword
    let mut inside_target_element = false;
    let mut element_name = String::new();
    let mut inside_body = false;

    while let Ok(event) = parser.next() {
        match event {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                // Check if <p> or <tr> is directly inside <w:body>
                if (path_stack.back() == Some(&"body".to_string()) && name.local_name == "p")
                    || name.local_name == "tr"
                {
                    inside_target_element = true;
                    element_name = name.local_name.clone();
                }

                // Add the element name to the path stack
                path_stack.push_back(name.local_name.clone());

                // Check if we're inside <w:body>
                if path_stack.len() == 2 && path_stack[0] == "body" && name.local_name == "p" {
                    inside_body = true;
                }

                let attributes: Vec<OwnedAttribute> = attributes
                    .iter()
                    .map(|attr| OwnedAttribute {
                        name: attr.name.clone(),
                        value: attr.value.clone(),
                    })
                    .collect();

                if inside_target_element {
                    let mut start_element = WriterXmlEvent::start_element(name.local_name.as_str());

                    for attr in &attributes {
                        start_element =
                            start_element.attr(attr.name.local_name.as_str(), attr.value.as_str());
                    }

                    writer.write(start_element)?;
                }
            }

            /* XmlEvent::Characters(text) => {
                 if inside_target_element {
                     accumulated_text.push_str(&text); // Accumulate text

                     if in_keyword || accumulated_text.contains("${") {
                         in_keyword = true;
                         if accumulated_text.contains("}") {
                             in_keyword = false;
                             let replaced_text = replace_keywords(&accumulated_text, data);
                             writer.write(WriterXmlEvent::characters(&replaced_text))?;
                             accumulated_text.clear();
                         }
                     } else {
                         writer.write(WriterXmlEvent::characters(&accumulated_text))?;
                         accumulated_text.clear();
                     }
                 } else {
                     main_output.extend(&temp_output);
                     main_output.extend(accumulated_text.bytes());
                     temp_output.clear();
                 }
             } */
            /*  XmlEvent::Characters(text) => {
                 if inside_target_element {
                     accumulated_text.push_str(&text); // Accumulate text

                     if in_keyword || accumulated_text.contains("${") {
                         in_keyword = true;
                         if accumulated_text.contains("}") {
                             in_keyword = false;
                             let replaced_text = replace_keywords(&accumulated_text, data);
                             writer.write(WriterXmlEvent::characters(&replaced_text))?;
                             accumulated_text.clear();
                         }
                     } else {
                         writer.write(WriterXmlEvent::characters(&accumulated_text))?;
                         accumulated_text.clear();
                     }
                 }
             } */
            XmlEvent::Characters(text) => {
                if inside_target_element {
                    for c in text.chars() {
                        accumulated_text.push(c); // Accumulate the current character

                        // Check if we're inside a keyword
                        println!("Characters: {} ", accumulated_text);
                        if in_keyword || accumulated_text.contains("${") {
                            in_keyword = true;

                            // Check if we have an equal number of opening and closing braces
                            let open_braces = accumulated_text.matches("${").count();
                            let close_braces = accumulated_text.matches("}").count();

                            if open_braces > 0 && close_braces > 0 && close_braces == open_braces {
                                // If the number of opening and closing braces match, process the keyword
                                in_keyword = false;
                                let replaced_text = replace_keywords(&accumulated_text, data);

                                // Write the replaced text to the writer
                                writer.write(WriterXmlEvent::characters(&replaced_text))?;

                                // Clear the accumulated text
                                accumulated_text.clear();
                            }
                        } else {
                            // If we're not inside a keyword, write the text directly
                            if accumulated_text.ends_with('$') {
                                // Do nothing for now, just wait for the next character(s) to complete the keyword
                            } else {
                                // If the text doesn't end with `$`, write the text directly
                                writer.write(WriterXmlEvent::characters(&accumulated_text))?;
                                accumulated_text.clear(); // Clear the buffer after writing
                            } // Clear the buffer after writing
                        }
                    }
                }
            }

            XmlEvent::EndElement { name } => {
                if inside_target_element {
                    writer.write(WriterXmlEvent::end_element())?;
                }

                if inside_target_element && name.local_name == element_name {
                    inside_target_element = false;
                    drop(writer);

                    let content = String::from_utf8(temp_output.clone())?;
                    println!("Conteúdo do elemento: {}", content);

                    main_output.extend(temp_output);
                    temp_output = Vec::new(); // Clear temp_output
                    writer = EmitterConfig::new()
                        .perform_indent(true)
                        .create_writer(&mut temp_output);
                }

                if let Some(last) = path_stack.pop_back() {
                    if last == "document" {
                        break;
                    }

                    if last == "body" {
                        inside_body = false;
                    }
                }
            }

            _ => {
                println!("Evento não tratado: {:?}", event);
            }
        }
    }

    main_output.extend(&temp_output); // Ensure anything left in temp_output is added
    Ok(String::from_utf8(main_output)?)
}
/* fn process_word_xml_with_substitutions(
    doc_xml: &str,
    data: &Value,
) -> Result<String, Box<dyn std::error::Error>> {
    let cursor = Cursor::new(doc_xml.as_bytes());
    let mut parser = EventReader::new(cursor);
    let mut path_stack: VecDeque<String> = VecDeque::new(); // To track the XML element path

    let mut main_output = Vec::new();
    let mut temp_output = Vec::new();
    let mut writer = EmitterConfig::new()
        .perform_indent(true)
        .create_writer(&mut temp_output);

    let mut accumulated_text = String::new();
    let mut in_keyword = false; // Flag indicating whether we're inside a keyword
    let mut inside_target_element = false;
    let mut element_name = String::new();
    let mut inside_body = false;

    while let Ok(event) = parser.next() {
        match event {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                // Check if <p> or <tr> is directly inside <w:body>
                if (path_stack.back() == Some(&"body".to_string()) && name.local_name == "p")
                    || name.local_name == "tr"
                {
                    inside_target_element = true;
                    element_name = name.local_name.clone();
                }

                // Add the element name to the path stack
                path_stack.push_back(name.local_name.clone());

                // Check if we're inside <w:body>
                if path_stack.len() == 2 && path_stack[0] == "body" && name.local_name == "p" {
                    inside_body = true;
                }

                let attributes: Vec<OwnedAttribute> = attributes
                    .iter()
                    .map(|attr| OwnedAttribute {
                        name: attr.name.clone(),
                        value: attr.value.clone(),
                    })
                    .collect();

                if inside_target_element {
                    let mut start_element = WriterXmlEvent::start_element(name.local_name.as_str());

                    for attr in &attributes {
                        start_element =
                            start_element.attr(attr.name.local_name.as_str(), attr.value.as_str());
                    }

                    writer.write(start_element)?;
                }
            }

            XmlEvent::Characters(text) => {
                if inside_target_element {
                    accumulated_text.push_str(&text); // Accumulate text

                    if in_keyword || accumulated_text.contains("${") {
                        in_keyword = true;
                        if accumulated_text.contains("}") {
                            in_keyword = false;
                            let replaced_text = replace_keywords(&accumulated_text, data);
                            writer.write(WriterXmlEvent::characters(&replaced_text))?;
                            accumulated_text.clear();
                        }
                    } else {
                        writer.write(WriterXmlEvent::characters(&accumulated_text))?;
                        accumulated_text.clear();
                    }
                } else {
                    main_output.append(&mut temp_output);
                    main_output.append(&mut accumulated_text.into_bytes());
                }
            }

            XmlEvent::EndElement { name } => {
                if inside_target_element {
                    writer.write(WriterXmlEvent::end_element())?;
                }

                if inside_target_element && name.local_name == element_name {
                    inside_target_element = false;
                    drop(writer);

                    let content = String::from_utf8(temp_output.clone())?;
                    println!("Conteúdo do elemento: {}", content);

                    main_output.append(&mut temp_output);
                    writer = EmitterConfig::new()
                        .perform_indent(true)
                        .create_writer(&mut temp_output);
                }

                if let Some(last) = path_stack.pop_back() {
                    if last == "document" {
                        break;
                    }

                    if last == "body" {
                        inside_body = false;
                    }
                }
            }

            _ => {
                println!("Evento não tratado: {:?}", event);
            }
        }
    }

    Ok(String::from_utf8(main_output)?)
}
 */
/*
fn process_word_xml_with_substitutions( doc_xml: &str,
    data: &Value,
) -> Result<String, Box<dyn std::error::Error>> {
    let cursor = Cursor::new(doc_xml.as_bytes());
    let mut parser = EventReader::new(cursor);
    let mut buffer = String::new();
    let mut inside_target_element = false; // Controle se estamos dentro de um <p> ou <tr>
    let mut element_name = String::new(); // Para armazenar o nome do elemento atual
    let mut path_stack: VecDeque<String> = VecDeque::new(); // Para rastrear o caminho no XML
    let mut inside_body = false;

    let mut output = Vec::new();
    let mut writer = EmitterConfig::new()
        .perform_indent(true)
        .create_writer(&mut output);

    let mut accumulated_text = String::new();
    let mut in_keyword = false; // Indica se estamos dentro de uma keyword

    while let Ok(event) = parser.next() {
        match event {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                // Verifica se <p> ou <tr> estão diretamente dentro de <w:body>
                if (path_stack.back() == Some(&"body".to_string()) && name.local_name == "p")
                    || name.local_name == "tr"
                {
                    inside_target_element = true;
                    element_name = name.local_name.clone();
                }

                // Adicionar o nome do elemento à pilha do caminho
                path_stack.push_back(name.local_name.clone());

                // Verifica se estamos dentro de <w:body>
                if path_stack.contains(&"body".to_string()) {
                    inside_body = true;
                }

                let attributes: Vec<OwnedAttribute> = attributes
                    .iter()
                    .map(|attr| OwnedAttribute {
                        name: attr.name.clone(),
                        value: attr.value.clone(),
                    })
                    .collect();

                if inside_target_element {
                    let mut start_element = WriterXmlEvent::start_element(name.local_name.as_str());

                    for attr in &attributes {
                        start_element =
                            start_element.attr(attr.name.local_name.as_str(), attr.value.as_str());
                    }

                    writer.write(start_element)?;
                }
            }

            XmlEvent::Characters(text) => {
                if inside_target_element {
                    accumulated_text.push_str(&text); // Acumula o texto

                    // Verifique se estamos no meio de uma keyword
                    if in_keyword || accumulated_text.contains("${") {
                        in_keyword = true;
                        if accumulated_text.contains("}") {
                            in_keyword = false;
                            let replaced_text = replace_keywords(&accumulated_text, data);
                            writer.write(WriterXmlEvent::characters(&replaced_text))?;
                            accumulated_text.clear(); // Clear after processing
                        }
                    } else {
                        writer.write(WriterXmlEvent::characters(&accumulated_text))?;
                        accumulated_text.clear(); // Clear after writing
                    }
                }
            }

            XmlEvent::EndElement { name } => {
                if inside_target_element {
                    writer.write(WriterXmlEvent::end_element())?;
                }

                if inside_target_element && name.local_name == element_name {
                    inside_target_element = false;
                    drop(writer);

                    let content = String::from_utf8(output.clone())?;
                    println!("Conteúdo do elemento: {}", content);

                    output.clear();
                    writer = EmitterConfig::new()
                        .perform_indent(true)
                        .create_writer(&mut output);
                }

                if let Some(last) = path_stack.pop_back() {
                    if last == "document" {
                        break;
                    }

                    if last == "body" {
                        inside_body = false;
                    }
                }
            }

            _ => {
                println!("Evento não tratado: {:?}", event);
            }
        }
    }

    Ok(String::from_utf8(output)?)
}
 */

/* fn replace_keywords(text: &str, data: &Value) -> String {
    let mut replaced_text = text.to_string();

    // Check if the data is an object and handle replacements
    if let Some(map) = data.as_object() {
        for (key, value) in map {
            let placeholder = format!("${{{}}}", key);
            if replaced_text.contains(&placeholder) {
                let replacement = match value {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => {
                        if *b {
                            "Verdadeiro".to_string()
                        } else {
                            "Falso".to_string()
                        }
                    }
                    Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
                    _ => "".to_string(),
                };
                replaced_text = replaced_text.replace(&placeholder, &replacement);
            }
        }
    }

    // Handle cases where data is a primitive (non-object) value
    if replaced_text.contains("${value}") {
        if let Some(replacement) = match data {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            Value::Bool(b) => Some(if *b { "Verdadeiro" } else { "Falso" }.to_string()),
            _ => None,
        } {
            replaced_text = replaced_text.replace("${value}", &replacement);
        }
    }

    replaced_text
}
 */
// Função para substituir keywords no texto
pub fn replace_keywords<T>(text: T, data: &Value) -> String
where
    T: AsRef<str> + Debug + ToString,
{
    let mut replaced_text = text.to_string();

    // Função auxiliar para acessar atributos em objetos aninhados
    fn get_nested_value<'a>(data: &'a Value, path: &str) -> Option<&'a Value> {
        let mut current_value = data;
        for part in path.split('.') {
            if let Value::Object(map) = current_value {
                current_value = map.get(part.trim())?; // Remove espaços em branco
            } else {
                return None;
            }
        }
        Some(current_value)
    }

    let locale = &*LOCALE;

    // Substituir todos os placeholders encontrados
    while let Some(start_pos) = replaced_text.find("${") {
        if let Some(end_pos) = replaced_text[start_pos..].find('}') {
            let placeholder = &replaced_text[start_pos + 2..start_pos + end_pos];
            let trimmed_placeholder = placeholder.trim(); // Remove espaços dentro do placeholder

            let (key, format) = if let Some(pipe_pos) = trimmed_placeholder.find('|') {
                let key = trimmed_placeholder[..pipe_pos].trim();
                let format_spec = &trimmed_placeholder[pipe_pos + 1..].trim();
                if format_spec.starts_with("date:") {
                    (key, &format_spec[5..])
                } else {
                    (trimmed_placeholder, "")
                }
            } else {
                (trimmed_placeholder, "")
            };

            let replacement = if key == "value" {
                match data {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => {
                        if *b {
                            TRANSLATIONS
                                .get(locale)
                                .and_then(|t| t.get("boolean"))
                                .and_then(|b| b.get("true"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("Verdadeiro")
                                .to_string()
                        } else {
                            TRANSLATIONS
                                .get(locale)
                                .and_then(|t| t.get("boolean"))
                                .and_then(|b| b.get("false"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("Falso")
                                .to_string()
                        }
                    }
                    _ => "".to_string(),
                }
            } else {
                // Tenta obter o valor para o caminho especificado
                if let Some(value) = get_nested_value(data, key) {
                    match value {
                        Value::String(s) => {
                            if format.is_empty() {
                                s.clone()
                            } else {
                                format_date_with_locale(s, format, locale)
                            }
                        }
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => {
                            if *b {
                                TRANSLATIONS
                                    .get(locale)
                                    .and_then(|t| t.get("boolean"))
                                    .and_then(|b| b.get("true"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Verdadeiro")
                                    .to_string()
                            } else {
                                TRANSLATIONS
                                    .get(locale)
                                    .and_then(|t| t.get("boolean"))
                                    .and_then(|b| b.get("false"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Falso")
                                    .to_string()
                            }
                        }
                        _ => "".to_string(),
                    }
                } else {
                    "".to_string() // Se não encontrar o valor, substitui por vazio
                }
            };

            replaced_text.replace_range(start_pos..start_pos + end_pos + 1, &replacement);
        } else {
            break;
        }
    }

    replaced_text
}
/* pub fn replace_keywords<T>(text: T, data: &Value) -> String
where
    T: AsRef<str> + Debug + ToString,
{
    let mut replaced_text = text.to_string();

    // Função auxiliar para acessar atributos em objetos aninhados
    fn get_nested_value<'a>(data: &'a Value, path: &str) -> Option<&'a Value> {
        let mut current_value = data;
        for part in path.split('.') {
            if let Value::Object(map) = current_value {
                current_value = map.get(part.trim())?; // Remove espaços em branco
            } else {
                return None;
            }
        }
        Some(current_value)
    }

    // Função auxiliar para formatar datas com suporte a localização
    fn format_date(date_str: &str, format: &str, locale: &str) -> String {
        let possible_formats = ["%Y-%m-%d", "%Y/%m/%d", "%d-%m-%Y", "%d/%m/%Y"];

        for &date_format in &possible_formats {
            if let Ok(date) = NaiveDate::parse_from_str(date_str, date_format) {
                match format {
                    "short" => return date.format_localized("%x", locale.into()).to_string(),
                    "long" => {
                        return date
                            .format_localized("%A, %d de %B de %Y", locale.into())
                            .to_string()
                    }
                    _ => return date.format_localized("%x", locale.into()).to_string(),
                }
            }
        }

        date_str.to_string() // Retorna a string original se o parsing falhar
    }

    // Obtém a localização do sistema operacional
    //let locale = get_locale().unwrap_or_else(|| "en_US".to_string());
    let locale = get_locale().unwrap_or_else(|| "pt_BR".to_string());

    // Substituir todos os placeholders encontrados
    while let Some(start_pos) = replaced_text.find("${") {
        if let Some(end_pos) = replaced_text[start_pos..].find('}') {
            let placeholder = &replaced_text[start_pos + 2..start_pos + end_pos];
            let trimmed_placeholder = placeholder.trim(); // Remove espaços dentro do placeholder

            let (key, format) = if let Some(pipe_pos) = trimmed_placeholder.find('|') {
                let key = trimmed_placeholder[..pipe_pos].trim();
                let format_spec = &trimmed_placeholder[pipe_pos + 1..].trim();
                if format_spec.starts_with("date:") {
                    (key, &format_spec[5..])
                } else {
                    (trimmed_placeholder, "")
                }
            } else {
                (trimmed_placeholder, "")
            };

            let replacement = if key == "value" {
                match data {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => {
                        if *b {
                            "Verdadeiro".to_string()
                        } else {
                            "Falso".to_string()
                        }
                    }
                    _ => "".to_string(),
                }
            } else {
                // Tenta obter o valor para o caminho especificado
                if let Some(value) = get_nested_value(data, key) {
                    match value {
                        Value::String(s) => {
                            if format.is_empty() {
                                s.clone()
                            } else {
                                format_date(s, format, &locale)
                            }
                        }
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => {
                            if *b {
                                "Verdadeiro".to_string()
                            } else {
                                "Falso".to_string()
                            }
                        }
                        _ => "".to_string(),
                    }
                } else {
                    "".to_string() // Se não encontrar o valor, substitui por vazio
                }
            };

            replaced_text.replace_range(start_pos..start_pos + end_pos + 1, &replacement);
        } else {
            break;
        }
    }

    replaced_text
}
 */
/* pub fn replace_keywords<T>(text: T, data: &Value) -> String
where
    T: AsRef<str> + Debug + ToString,
{

    let mut replaced_text = text.to_string();

    // Função auxiliar para acessar atributos em objetos aninhados
    fn get_nested_value<'a>(data: &'a Value, path: &str) -> Option<&'a Value> {
        let mut current_value = data;
        for part in path.split('.') {
            if let Value::Object(map) = current_value {
                current_value = map.get(part.trim())?; // Remove espaços em branco
            } else {
                return None;
            }
        }
        Some(current_value)
    }

    // Função auxiliar para formatar datas
    fn format_date(date_str: &str, format: &str) -> String {
        if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            match format {
                "short" => date.format("%d/%m/%Y").to_string(),
                "long" => date.format("%d de %B de %Y").to_string(),
                "month_year" => date.format("%B de %Y").to_string(),
                "year" => date.format("%Y").to_string(),
                _ => date.format("%d/%m/%Y").to_string(), // Formato padrão
            }
        } else {
            date_str.to_string() // Retorna a string original se o parsing falhar
        }
    }

    // Substituir todos os placeholders encontrados
    while let Some(start_pos) = replaced_text.find("${") {
        if let Some(end_pos) = replaced_text[start_pos..].find('}') {
            let placeholder = &replaced_text[start_pos + 2..start_pos + end_pos];
            let trimmed_placeholder = placeholder.trim(); // Remove espaços dentro do placeholder

            let (key, format) = if let Some(pipe_pos) = trimmed_placeholder.find('|') {
                let key = trimmed_placeholder[..pipe_pos].trim();
                let format_spec = &trimmed_placeholder[pipe_pos + 1..].trim();
                if format_spec.starts_with("date:") {
                    (key, &format_spec[5..])
                } else {
                    (trimmed_placeholder, "")
                }
            } else {
                (trimmed_placeholder, "")
            };

            let replacement = if key == "value" {
                match data {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => {
                        if *b {
                            "Verdadeiro".to_string()
                        } else {
                            "Falso".to_string()
                        }
                    }
                    _ => "".to_string(),
                }
            } else {
                // Tenta obter o valor para o caminho especificado
                if let Some(value) = get_nested_value(data, key) {
                    match value {
                        Value::String(s) => {
                            if format.is_empty() {
                                s.clone()
                            } else {
                                format_date(s, format)
                            }
                        }
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => {
                            if *b {
                                "Verdadeiro".to_string()
                            } else {
                                "Falso".to_string()
                            }
                        }
                        _ => "".to_string(),
                    }
                } else {
                    "".to_string() // Se não encontrar o valor, substitui por vazio
                }
            };

            replaced_text.replace_range(start_pos..start_pos + end_pos + 1, &replacement);
        } else {
            break;
        }
    }

    replaced_text
}
 */
/* pub fn replace_keywords<T>(text: T, data: &Value) -> String
where
    T: AsRef<str> + Debug + ToString,
{
    let mut replaced_text = text.to_string();

    // Função auxiliar para acessar atributos em objetos aninhados
    fn get_nested_value<'a>(data: &'a Value, path: &str) -> Option<&'a Value> {
        let mut current_value = data;
        for part in path.split('.') {
            if let Value::Object(map) = current_value {
                current_value = map.get(part.trim())?; // Remove espaços em branco
            } else {
                return None;
            }
        }
        Some(current_value)
    }

    // Função auxiliar para formatar datas
    fn format_date(date_str: &str, format: &str) -> String {
        if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            match format {
                "short" => date.format("%d/%m/%Y").to_string(),
                "long" => format!(
                    "{} de {} de {}",
                    date.day(),
                    month_name(date.month()),
                    date.year()
                ),
                _ => date.to_string(),
            }
        } else {
            date_str.to_string() // Se a data não puder ser analisada, retorna como está
        }
    }

    // Função auxiliar para retornar o nome do mês
    fn month_name(month: u32) -> &'static str {
        match month {
            1 => "janeiro",
            2 => "fevereiro",
            3 => "março",
            4 => "abril",
            5 => "maio",
            6 => "junho",
            7 => "julho",
            8 => "agosto",
            9 => "setembro",
            10 => "outubro",
            11 => "novembro",
            12 => "dezembro",
            _ => "",
        }
    }

    // Substituir todos os placeholders encontrados
    while let Some(start_pos) = replaced_text.find("${") {
        if let Some(end_pos) = replaced_text[start_pos..].find('}') {
            let placeholder = &replaced_text[start_pos + 2..start_pos + end_pos];
            let trimmed_placeholder = placeholder.trim(); // Remove espaços dentro do placeholder

            let (key, format) = if let Some((key, format)) = trimmed_placeholder.split_once(':') {
                (key.trim(), format.trim())
            } else {
                (trimmed_placeholder, "")
            };

            let replacement = if key == "value" {
                match data {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => {
                        if *b {
                            "Verdadeiro".to_string()
                        } else {
                            "Falso".to_string()
                        }
                    }
                    _ => "".to_string(),
                }
            } else {
                if let Some(value) = get_nested_value(data, key) {
                    match value {
                        Value::String(s) => {
                            if format.is_empty() {
                                s.clone()
                            } else {
                                format_date(s, format)
                            }
                        }
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => {
                            if *b {
                                "Verdadeiro".to_string()
                            } else {
                                "Falso".to_string()
                            }
                        }
                        _ => "".to_string(),
                    }
                } else {
                    "".to_string() // Se não encontrar o valor, substitui por vazio
                }
            };

            replaced_text.replace_range(start_pos..start_pos + end_pos + 1, &replacement);
        } else {
            break;
        }
    }

    replaced_text
}
 */
/* pub fn replace_keywords<T>(text: T, data: &Value) -> String
where
    T: AsRef<str> + Debug + ToString,
{
    let mut replaced_text = text.to_string();

    //eprintln!("Initial text: {}", replaced_text);
    //eprintln!("Data received: {:?}", data);

    if let Some(map) = data.as_object() {
        //eprintln!("Processing as an object with {} keys.", map.len());
        for (key, value) in map {
            let placeholder = format!("${{{}}}", key);
            //eprintln!("Checking for placeholder: {}", placeholder);
            if replaced_text.contains(&placeholder) {
                eprintln!("Placeholder {} found!", placeholder);
                let replacement = match value {
                    Value::String(s) => {
                        //eprintln!("Replacing with string value: {}", s);
                        s.clone()
                    }
                    Value::Number(n) => {
                        let num_str = n.to_string();
                        //eprintln!("Replacing with number value: {}", num_str);
                        num_str
                    }
                    Value::Bool(b) => {
                        let bool_str = if *b { "Verdadeiro" } else { "Falso" }.to_string();
                        //eprintln!("Replacing with boolean value: {}", bool_str);
                        bool_str
                    }
                    Value::Object(_) => {
                        let obj_str = serde_json::to_string(value).unwrap_or_default();
                        //eprintln!("Replacing with object string: {}", obj_str);
                        obj_str
                    }
                    _ => {
                        eprintln!("Replacing with empty string.");
                        "".to_string()
                    }
                };
                replaced_text = replaced_text.replace(&placeholder, &replacement);
                //eprintln!("Replaced text: {}", replaced_text);
            }
        }
    } else {
        //eprintln!("Processing as a primitive type.");
        if replaced_text.contains("${value}") {
            if let Some(replacement) = match data {
                Value::String(s) => {
                    //eprintln!("Replacing with primitive string value: {}", s);
                    Some(s.clone())
                }
                Value::Number(n) => {
                    let num_str = n.to_string();
                    //eprintln!("Replacing with primitive number value: {}", num_str);
                    Some(num_str)
                }
                Value::Bool(b) => {
                    let bool_str = if *b { "Verdadeiro" } else { "Falso" }.to_string();
                    //eprintln!("Replacing with primitive boolean value: {}", bool_str);
                    Some(bool_str)
                }
                _ => {
                    //eprintln!("No replacement for this primitive type.");
                    None
                }
            } {
                replaced_text = replaced_text.replace("${value}", &replacement);
                //eprintln!("Replaced text with primitive value: {}", replaced_text);
            }
        }
    }

    //eprintln!("Final replaced text: {}", replaced_text);
    replaced_text
} */

/* fn replace_keywords(text: &str, data: &Value) -> String {
    let mut replaced_text = text.to_string();

    // Handle when `data` is an object
    if let Some(map) = data.as_object() {
        println!("obj: {:?} map: {:?}", data, map);
        for (key, value) in map {
            let placeholder = format!("${{{}}}", key);
            println!(
                "placeholder:{:?} key:{:?} value:{:?}",
                placeholder, key, value
            );
            if replaced_text.contains(&placeholder) {
                let replacement = match value {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => {
                        if *b {
                            "Verdadeiro".to_string()
                        } else {
                            "Falso".to_string()
                        }
                    }
                    Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
                    _ => "".to_string(),
                };
                replaced_text = replaced_text.replace(&placeholder, &replacement);
            }
        }
    } else {
        // Handle when `data` is a primitive type (string, number, boolean)
        if let Some(replacement) = match data {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            Value::Bool(b) => Some(if *b { "Verdadeiro" } else { "Falso" }.to_string()),
            _ => None,
        } {
            replaced_text = replaced_text.replace("${value}", &replacement);
        }
    }

    replaced_text
}
 */
// src/variable_substitution.rs

/// Handles the substitution of variables within the XML content.
///
/// # Arguments
///
/// * `content` - The XML content as a string where the substitution needs to occur.
/// * `data` - A reference to the data that contains the values to be substituted.
///
/// # Returns
///
/// A `Result` containing the substituted content as a string, or an error if the substitution fails.
pub fn substitute_variable(
    content: &str,
    data: &serde_json::Value,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut substituted_content = content.to_string();

    // Example: Replace ${value} with the data if it's a string, int, or boolean
    if let Some(value) = data.get("value") {
        let replacement = match value {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => {
                if *b {
                    "Verdadeiro".to_string()
                } else {
                    "Falso".to_string()
                }
            }
            _ => String::new(),
        };
        substituted_content = substituted_content.replace("${value}", &replacement);
    }

    // Example: Replace ${keyword} with corresponding value from the data object
    if let Some(object) = data.as_object() {
        for (key, value) in object {
            if let Some(replacement) = value.as_str() {
                substituted_content =
                    substituted_content.replace(&format!("${{{}}}", key), replacement);
            }
        }
    }

    Ok(substituted_content)
}

/// Handles substitution when the value is an object.
///
/// # Arguments
///
/// * `content` - The content in which to substitute variables.
/// * `object` - The JSON object containing the data for substitution.
///
/// # Returns
///
/// A `Result` containing the substituted content or an error.
pub fn handle_object_substitution(
    content: &str,
    object: &serde_json::Map<String, serde_json::Value>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut substituted_content = content.to_string();

    for (key, value) in object {
        let replacement = match value {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => {
                if *b {
                    "Verdadeiro".to_string()
                } else {
                    "Falso".to_string()
                }
            }
            _ => String::new(),
        };
        substituted_content = substituted_content.replace(&format!("${{{}}}", key), &replacement);
    }

    Ok(substituted_content)
}

/// Detects keywords in the format `${keyword}` and replaces them with corresponding values.
///
/// # Arguments
///
/// * `content` - The XML content where the substitution will happen.
/// * `keyword` - The keyword to be replaced.
/// * `replacement` - The replacement string for the keyword.
///
/// # Returns
///
/// The content with the keyword replaced by the replacement value.
pub fn detect_and_replace_keyword(content: &str, keyword: &str, replacement: &str) -> String {
    content.replace(&format!("${{{}}}", keyword), replacement)
}

/// Unit tests for the `variable_substitution` module.

#[cfg(test)]
mod tests_variables_substituicoes {
    use crate::utils::extract_document_xml;

    use super::*;
    use serde_json::json;
    use serde_json::Value;

    #[test]
    fn test_substitute_variable_with_string() {
        // let content = "Hello, ${value}!";
        let value = serde_json::json!("Worldj");
        let docx_path = "tests/modelo.dotx"; // path to your test DOCX file
        let content = extract_document_xml(docx_path).unwrap();
        //println!("content:{:?}", content);

        let result = process_word_xml_with_substitutions(content.as_str(), &value).unwrap();
        //assert_eq!(result, "Hello, World!");
        println!("RESULT:{:?}", result);
        assert!(result.contains("Worldj!"));
    }

    #[test]
    fn test_substitute_variable_with_number() {
        //let content = "The answer is ${value}.";
        let docx_path = "tests/modelo.dotx"; // path to your test DOCX file
        let content = extract_document_xml(docx_path).unwrap();
        let value = Value::Number(4278.into());
        let result = process_word_xml_with_substitutions(content.as_str(), &value).unwrap();
        //assert_eq!(result, "The answer is 42.");
        println!("RESULT:{:?}", result);
        assert!(result.contains("4278."));
    }
    #[test]
    fn test_substitute_variable_with_boolean() {
        //let content = "Is it true? ${value}.";
        let docx_path = "tests/modelo.dotx"; // path to your test DOCX file
        let content = extract_document_xml(docx_path).unwrap();
        let value = Value::Bool(true);
        let result = process_word_xml_with_substitutions(content.as_str(), &value).unwrap();
        assert!(result.contains("Verdadeiro."));
        //assert_eq!(result, "Is it true? Verdadeiro.");

        let value = Value::Bool(false);
        let result = process_word_xml_with_substitutions(content.as_str(), &value).unwrap();
        //assert_eq!(result, "Is it true? Falso.");
        println!("RESULT:{:?}", result);
        assert!(result.contains("Falso."));
    }

    #[test]
    fn test_substitute_variable_with_object() {
        let docx_path = "tests/modelo.dotx"; // path to your test DOCX file
        let content = extract_document_xml(docx_path).unwrap();
        //let content = "Hello, ${name}. You are ${age} years old.";
        let value = json!({
            "name": "Alice",
            "age": 30,
            "mulher": true
        });
        let result = process_word_xml_with_substitutions(content.as_str(), &value).unwrap();
        println!("RESULT:{:?}", result);
        assert!(result.contains("Hello, Alice. You are 30 years"));
    }

    #[test]
    fn test_substitute_variable_with_nested_object() {
        let docx_path = "tests/modelo.dotx"; // path to your test DOCX file
        let content = extract_document_xml(docx_path).unwrap();

        let value = json!({
            "partes": {
                "requerente": {
                    "nome": "Alice",
                    "data_de_nascimento": {
                        "dia": 15,
                        "mes": "Junho",
                        "ano": 1990
                    }
                }
            },
            "mulher": true
        });

        let result = process_word_xml_with_substitutions(content.as_str(), &value).unwrap();
        println!("RESULTADO: {:?}", result);

        assert!(result.contains("Alice"));
        assert!(result.contains("15"));
        assert!(result.contains("Junho"));
        assert!(result.contains("1990"));
        assert!(result.contains("Verdadeiro"));
    }

    #[test]
    fn test_substitute_variable_with_object_and_date() {
        let docx_path = "tests/modelo.dotx"; // path to your test DOCX file
        let content = extract_document_xml(docx_path).unwrap();

        let value = json!({
            "partes": {
                "requerente": {
                    "nome": "João Silva",
                    "data_nascimento": "1990-05-15"
                }
            },
            "data_registro": "2024-06-01"
        });

        let result = replace_keywords(content.as_str(), &value);

        println!("Resultado: {:?}", result);

        assert!(result.contains("João Silva"));
        assert!(result.contains("15/05/1990")); // Data curta
        assert!(result.contains("1 de junho de 2024")); // Data longa
    }
}
