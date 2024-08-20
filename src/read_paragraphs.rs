use serde_json::Value;

use std::borrow::Cow;
use std::collections::VecDeque;
use std::io::Cursor;
use xml::attribute::{Attribute, OwnedAttribute};
use xml::name::Name; // Certifique-se de que estes estejam importados corretamente // Importação correta para Attribute e OwnedAttribute
use xml::reader::{EventReader, XmlEvent};
use xml::writer::{EmitterConfig, XmlEvent as WriterXmlEvent}; // Importar para utilizar como pilha

pub fn modify_paragraphs_in_xml(
    doc_xml: &str,
    data: &Value,
) -> Result<String, Box<dyn std::error::Error>> {
    let doc_xml_modificado = process_xml(doc_xml);

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

    let mut in_variable = false; // Adicionando a variável in_variable
    let mut repeating_paragraphs: Vec<XmlEvent> = Vec::new(); // Buffer para capturar parágrafos dentro do bloco de repetição
    let mut block_key = String::new(); // Para armazenar a chave do bloco de repetição

    let mut element_stack: VecDeque<String> = VecDeque::new(); // Inicializar a pilha

    // Imprime o XML original antes de qualquer modificação
    println!("XML original:\n{}", doc_xml);
    for e in parser {
        match e? {
            /* XmlEvent::StartElement {
                           name,
                           attributes,
                           namespace,
                       } => {
                           println!("StartElement: {}", name.local_name);

                           // Empilhar o elemento
                           element_stack.push_back(name.local_name.clone());

                           if name.local_name == "p" {
                               in_paragraph = true;
                               paragraph_modified = false;
                               paragraph_text.clear(); // Limpar o buffer para o novo parágrafo
                               keyword.clear(); // Limpar a keyword para o novo parágrafo
                                                //repeating_paragraphs.clear();
                           }

                           let owned_attributes: Vec<OwnedAttribute> = attributes
                               .iter()
                               .map(|attr| OwnedAttribute {
                                   name: attr.name.clone(),
                                   value: attr.value.clone(),
                               })
                               .collect();

                           let name_borrowed: Name = match (name.prefix.as_deref(), name.namespace.as_deref())
                           {
                               (Some(prefix), Some(namespace)) => {
                                   Name::qualified(namespace, &name.local_name, Some(prefix))
                               }
                               (None, Some(namespace)) => Name::qualified(namespace, &name.local_name, None),
                               _ => Name::local(&name.local_name),
                           };

                           // Convertendo para Vec<Attribute> ao invés de OwnedAttribute
                           let borrowed_attributes: Vec<Attribute> = attributes
                               .iter()
                               .map(|attr| Attribute {
                                   name: attr.name.borrow(),
                                   value: attr.value.as_str(),
                               })
                               .collect();

                           if in_repeating_block {
                               repeating_paragraphs.push(XmlEvent::StartElement {
                                   name: name_borrowed.clone().into(),
                                   attributes: owned_attributes.clone(), // Clone os atributos
                                   namespace: namespace.clone(),
                               });
                           } else if !in_keyword {
                               writer.write(WriterXmlEvent::StartElement {
                                   name: name_borrowed,
                                   attributes: Cow::Owned(borrowed_attributes), // Usando Cow::Owned com Vec<Attribute>

                                   namespace: Cow::Owned(namespace.clone()),
                               })?;
                           }
                       }
            */
            XmlEvent::StartElement {
                name,
                attributes,
                namespace,
            } => {
                // Imprimir o nome do elemento antes de modificar
                //println!("StartElement - name: {}", name.local_name);
                println!("StartElement: {}", name.local_name);

                // Empilhar o elemento
                element_stack.push_back(name.local_name.clone());

                if name.local_name == "p" {
                    in_paragraph = true;
                    paragraph_modified = false;
                    paragraph_text.clear(); // Limpar o buffer para o novo parágrafo
                    keyword.clear(); // Limpar a keyword para o novo parágrafo

                    /* let events = EventReader::new(doc_xml.as_bytes()); */

                    // Cria um novo EventReader para processar o conteúdo do parágrafo
                    let cursor = Cursor::new(doc_xml.as_bytes());
                    let sub_parser = EventReader::new(cursor);

                    // Processa o parágrafo usando o novo EventReader
                    let paragraph_content =
                        process_paragraph(&name.local_name, attributes.clone(), sub_parser)?;

                    // Agora, `paragraph_content` contém todo o texto e elementos filhos do parágrafo
                    println!("Conteúdo do parágrafo: {}", paragraph_content);

                    // Verificar se deve ser incluído em repeating_paragraphs
                    if should_include_in_repeating_block(&paragraph_content) {
                        repeating_paragraphs.push(XmlEvent::Characters(paragraph_content.clone()));
                    } else {
                        writer.write(WriterXmlEvent::Characters(&paragraph_content))?;
                    }
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

            /*    XmlEvent::Characters(text) => {
                                      if text.contains("${#") {
                                          in_repeating_block = true;
                                          block_key = text.clone();
                                          repeating_paragraphs.clear();
                                      } else if text.contains("${/") {
                                          in_repeating_block = false;
                                          let key = &block_key[3..block_key.len() - 1];
                                          if let Some(Value::Array(items)) = data.get(key) {
                                              for item in items {
                                                  for event in &repeating_paragraphs {
                                                      match event {
                                                          XmlEvent::StartElement {
                                                              name,
                                                              attributes,
                                                              namespace,
                                                          } => {
                                                              let borrowed_attributes: Vec<Attribute> = attributes
                                                                  .iter()
                                                                  .map(|attr| Attribute {
                                                                      name: attr.name.borrow(),
                                                                      value: attr.value.as_str(),
                                                                  })
                                                                  .collect();

                                                              writer.write(WriterXmlEvent::StartElement {
                                                                  name: Name::local(&name.local_name),
                                                                  attributes: Cow::Owned(borrowed_attributes),
                                                                  namespace: Cow::Owned(namespace.clone()),
                                                              })?;
                                                          }
                                                          XmlEvent::Characters(text) => {
                                                              let mut new_text = text.clone();
                                                              if let Some(Value::String(value)) = item.get("item") {
                                                                  new_text = new_text.replace("${item}", value);
                                                              }
                                                              writer.write(WriterXmlEvent::Characters(&new_text))?;
                                                          }
                                                          XmlEvent::EndElement { name } => {
                                                              writer.write(WriterXmlEvent::EndElement {
                                                                  name: Some(Name::local(&name.local_name)),
                                                              })?;
                                                          }
                                                          _ => {}
                                                      }
                                                  }
                                              }
                                          }
                                          repeating_paragraphs.clear();
                                      } else if in_repeating_block {
                                          repeating_paragraphs.push(XmlEvent::Characters(text.clone()));
                                      } else {
                                          if in_paragraph {
                                              paragraph_text.push_str(&text);

                                              for c in text.chars() {
                                                  if in_keyword {
                                                      keyword.push(c);
                                                      if keyword.ends_with("}") {
                                                          let key = &keyword[2..keyword.len() - 1];
                                                          let mut current_value: Option<&Value> = Some(data);
                                                          for part in key.split('.') {
                                                              if let Some(Value::Object(map)) = current_value {
                                                                  current_value = map.get(part);
                                                              } else {
                                                                  current_value = None;
                                                                  break;
                                                              }
                                                          }
                                                          if let Some(value) = current_value {
                                                              match value {
                                                                  serde_json::Value::String(value_str) => {
                                                                      writer
                                                                          .write(WriterXmlEvent::Characters(value_str))?;
                                                                  }
                                                                  serde_json::Value::Number(num) => {
                                                                      let value_str = num.to_string();
                                                                      writer.write(WriterXmlEvent::Characters(
                                                                          &value_str,
                                                                      ))?;
                                                                  }
                                                                  serde_json::Value::Bool(b) => {
                                                                      let value_str = b.to_string();
                                                                      writer.write(WriterXmlEvent::Characters(
                                                                          &value_str,
                                                                      ))?;
                                                                  }
                                                                  _ => {}
                                                              }
                                                          }
                                                          in_keyword = false;
                                                          keyword.clear();
                                                      }
                                                  } else {
                                                      if c == '$' {
                                                          keyword.push(c);
                                                      } else if keyword == "$" && c == '{' {
                                                          keyword.push(c);
                                                          in_keyword = true;
                                                      } else {
                                                          if !keyword.is_empty() {
                                                              writer.write(WriterXmlEvent::Characters("$"))?;
                                                              keyword.clear();
                                                          }
                                                          writer.write(WriterXmlEvent::Characters(&c.to_string()))?;
                                                      }
                                                  }
                                              }
                                          }
                                      }
                                  }
                       */
              /*          XmlEvent::Characters(text) => {
                           if in_paragraph {
                               paragraph_text.push_str(&text); // Acumular o texto do parágrafo

                               for c in text.chars() {
                                   if in_keyword {
                                       keyword.push(c);

                                       if c == '}' {
                                           if in_repeating_block && block_key.is_empty() {
                                               // Fechamento da abertura de bloco de repetição
                                               block_key = keyword.clone(); // Armazena o nome do bloco
                                           } else if in_repeating_block && !block_key.is_empty() {
                                               // Fechamento do bloco de repetição
                                               let key = &block_key[3..block_key.len() - 1]; // Extrai a chave do bloco de repetição

                                               if let Some(Value::Array(items)) = data.get(key) {
                                                   for item in items {
                                                       for event in &repeating_paragraphs {
                                                           match event {
                                                               XmlEvent::StartElement {
                                                                   name,
                                                                   attributes,
                                                                   namespace,
                                                               } => {
                                                                   let borrowed_attributes: Vec<Attribute> =
                                                                       attributes
                                                                           .iter()
                                                                           .map(|attr| Attribute {
                                                                               name: attr.name.borrow(),
                                                                               value: attr.value.as_str(),
                                                                           })
                                                                           .collect();

                                                                   writer.write(
                                                                       WriterXmlEvent::StartElement {
                                                                           name: Name::local(&name.local_name),
                                                                           attributes: Cow::Owned(
                                                                               borrowed_attributes,
                                                                           ),
                                                                           namespace: Cow::Owned(
                                                                               namespace.clone(),
                                                                           ),
                                                                       },
                                                                   )?;
                                                               }
                                                               XmlEvent::Characters(text) => {
                                                                   let mut new_text = text.clone();
                                                                   if let Some(Value::String(value)) =
                                                                       item.get("item")
                                                                   {
                                                                       new_text =
                                                                           new_text.replace("${item}", value);
                                                                   }
                                                                   writer.write(
                                                                       WriterXmlEvent::Characters(&new_text),
                                                                   )?;
                                                               }
                                                               XmlEvent::EndElement { name } => {
                                                                   writer.write(
                                                                       WriterXmlEvent::EndElement {
                                                                           name: Some(Name::local(
                                                                               &name.local_name,
                                                                           )),
                                                                       },
                                                                   )?;
                                                               }
                                                               _ => {}
                                                           }
                                                       }
                                                   }
                                               }
                                               repeating_paragraphs.clear();
                                               block_key.clear(); // Limpa o block_key após fechar o bloco
                                               in_repeating_block = false; // Sinaliza que o bloco foi fechado
                                           } else if in_variable {
                                               // Fechamento de uma variável ou atributo
                                               let key = &keyword[2..keyword.len() - 1];
                                               let mut current_value: Option<&Value> = Some(data);
                                               for part in key.split('.') {
                                                   if let Some(Value::Object(map)) = current_value {
                                                       current_value = map.get(part);
                                                   } else {
                                                       current_value = None;
                                                       break;
                                                   }
                                               }
                                               if let Some(value) = current_value {
                                                   match value {
                                                       serde_json::Value::String(value_str) => {
                                                           writer
                                                               .write(WriterXmlEvent::Characters(value_str))?;
                                                       }
                                                       serde_json::Value::Number(num) => {
                                                           let value_str = num.to_string();
                                                           writer.write(WriterXmlEvent::Characters(
                                                               &value_str,
                                                           ))?;
                                                       }
                                                       serde_json::Value::Bool(b) => {
                                                           let value_str = b.to_string();
                                                           writer.write(WriterXmlEvent::Characters(
                                                               &value_str,
                                                           ))?;
                                                       }
                                                       _ => {}
                                                   }
                                               }
                                           }

                                           // Resetando as flags e variáveis ao fechar uma keyword
                                           in_keyword = false;
                                           in_variable = false;
                                           keyword.clear();
                                       }
                                   } else {
                                       if c == '$' {
                                           keyword.push(c);
                                       } else if keyword == "$" && c == '{' {
                                           keyword.push(c);
                                           in_keyword = true;

                                           if paragraph_text.contains("${#") {
                                               in_repeating_block = true; // Identifica início de um bloco de repetição
                                           } else if paragraph_text.contains("${/") {
                                               in_repeating_block = false; // Identifica fechamento de um bloco de repetição
                                           } else {
                                               in_variable = true; // Identifica uma variável ou atributo
                                           }
                                       } else {
                                           if !keyword.is_empty() {
                                               writer.write(WriterXmlEvent::Characters("$"))?;
                                               keyword.clear();
                                           }

                                           if in_repeating_block {
                                               repeating_paragraphs.push(XmlEvent::Characters(c.to_string()));
                                           } else {
                                               writer.write(WriterXmlEvent::Characters(&c.to_string()))?;
                                           }
                                       }
                                   }
                               }
                           }
                       }
            */
            /* XmlEvent::EndElement { name } => {
                println!("EndElement: {}", name.local_name);

                if let Some(last_opened) = element_stack.pop_back() {
                    if last_opened != name.local_name {
                        println!(
                            "Erro: Tentando fechar '{}', mas o último elemento aberto foi '{}'",
                            name.local_name, last_opened
                        );

                        // Fecha todos os elementos abertos até encontrar o correspondente
                        while let Some(unmatched) = element_stack.pop_back() {
                            writer.write(WriterXmlEvent::EndElement {
                                name: Some(Name::local(&unmatched)),
                            })?;
                            println!("Fechando elemento não correspondido: {}", unmatched);

                            if unmatched == name.local_name {
                                break;
                            }
                        }
                    }
                } else {
                    println!(
                        "Erro: Tentando fechar '{}', mas a pilha de elementos está vazia",
                        name.local_name
                    );
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "End element with no matching start element",
                    )));
                }

                if in_keyword {
                    // Ignora o fechamento do elemento se ainda estamos em uma keyword
                } else if in_repeating_block {
                    // Se estamos em um bloco de repetição, armazena o evento de fechamento
                    repeating_paragraphs.push(XmlEvent::EndElement { name: name.clone() });
                } else {
                    writer.write(WriterXmlEvent::EndElement {
                        name: Some(Name::local(&name.local_name)),
                    })?;
                }

                if in_paragraph && name.local_name == "p" {
                    in_paragraph = false;
                    println!("Parágrafo que correspondeu à regex: {}", paragraph_text);
                }
            }
             */
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
                    //println!("Parágrafo que correspondeu à regex: {}", paragraph_text);
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

fn should_include_in_repeating_block(paragraph_text: &str) -> bool {
    println!("paragraph_text: {}", paragraph_text);
    paragraph_text.contains("${#") || paragraph_text.contains("${/}")
}

fn process_paragraph(
    element_name: &str,
    attributes: Vec<OwnedAttribute>,
    mut parser: EventReader<Cursor<&[u8]>>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut content = String::new();
    let mut depth = 1; // Inicializa com 1 para representar a tag de abertura que já foi processada.
    let mut inside_paragraph = true; // Começa dentro do parágrafo.

    // Adiciona a tag inicial ao conteúdo
    //content.push_str(&format!("<{} ", element_name));
    /* for attr in &attributes {
        content.push_str(&format!("{}=\"{}\" ", attr.name.local_name, attr.value));
    }
    content.push_str(">"); */

    while let Ok(event) = parser.next() {
        match event {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                if name.local_name == element_name {
                    depth += 1;
                }
                if inside_paragraph {
                    //content.push_str(&format!("<{}", name.local_name));
                    for attr in attributes {
                        //content.push_str(&format!(" {}=\"{}\"", attr.name.local_name, attr.value));
                    }
                    //content.push_str(">");
                }
                //println!("StartElement: {} - {}", content, depth);
            }
            XmlEvent::EndElement { name } => {
                if name.local_name == element_name {
                    depth -= 1;
                    if depth == 1 {
                        //content.push_str(&format!("</{}>", name.local_name));
                        inside_paragraph = false;
                        break; // Sai do loop ao encontrar o fechamento correspondente.
                    }
                }
            }
            XmlEvent::Characters(text) => {
                if inside_paragraph {
                    content.push_str(&text);
                }
            }
            _ => {}
        }
    }

    println!("Conteúdo final do parágrafo: {}", content);

    Ok(content)
}

fn process_element(
    element_name: &str,
    attributes: Vec<OwnedAttribute>,
    mut parser: EventReader<Cursor<Vec<u8>>>, // Mude para Cursor<Vec<u8>>
) -> Result<String, Box<dyn std::error::Error>> {
    let mut content = String::new();
    let mut depth = 1;

    while let Ok(event) = parser.next() {
        match event {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                content.push_str(&format!("<{}", name.local_name));
                for attr in attributes {
                    content.push_str(&format!(" {}=\"{}\"", attr.name.local_name, attr.value));
                }
                content.push_str(">");

                if name.local_name == element_name {
                    depth += 1;
                }
            }
            XmlEvent::EndElement { name } => {
                content.push_str(&format!("</{}>", name.local_name));

                if name.local_name == element_name {
                    depth -= 1;
                }
                if depth == 0 {
                    break;
                }
            }
            XmlEvent::Characters(text) => {
                content.push_str(&text);
            }
            _ => {}
        }
    }

    Ok(content)
}

fn handle_start_element(
    name: &xml::name::OwnedName,
    attributes: Vec<OwnedAttribute>,
    doc_xml: &str,
    buffer: &mut String,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    match name.local_name.as_str() {
        "p" | "tr" => {
            // Process the <p> or <tr> element
            let cursor = Cursor::new(doc_xml.as_bytes().to_vec()); // Create a Vec<u8> instead of &[u8]
            let sub_parser = EventReader::new(cursor);

            let element_content = process_element(&name.local_name, attributes, sub_parser)?;

            // Store the element content in the buffer
            buffer.push_str(&element_content);

            Ok(Some(element_content))
        }
        _ => Ok(None),
    }
}

fn handle_end_element(
    name: &xml::name::OwnedName,
    buffer: &mut String,
) -> Result<Option<Cursor<Vec<u8>>>, Box<dyn std::error::Error>> {
    match name.local_name.as_str() {
        "p" | "tr" => {
            // When an end element for <p> or <tr> is found, we create a Cursor from the buffer
            let cursor = Cursor::new(buffer.clone().into_bytes());
            Ok(Some(cursor))
        }
        _ => Ok(None),
    }
}

fn process_xml(doc_xml: &str) -> Result<(), Box<dyn std::error::Error>> {
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

    println!("process_xml");

    println!("XML original em process_xml:\n{}", doc_xml);

    while let Ok(event) = parser.next() {
        match event {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                // Verifica se <p> ou <tr> estão diretamente dentro de <w:body>
                if (path_stack.back() == Some(&"body".to_string()) && name.local_name == "p")
                    || (name.local_name == "tr")
                {
                    inside_target_element = true;
                    element_name = name.local_name.clone();

                    println!("inside_body && name.local_name == p \n{}", name.local_name);
                }

                println!(
                    "Nível: {} | Último no path_stack: {:?} | Elemento atual: {}",
                    path_stack.len(),
                    path_stack.back(),
                    name.local_name
                );

                // Adicionar o nome do elemento à pilha do caminho
                path_stack.push_back(name.local_name.clone());

                // Verifica se estamos dentro de <w:body>
                if path_stack.contains(&"body".to_string()) {
                    inside_body = true;
                }

                let attributes: Vec<Attribute> = attributes
                    .iter()
                    .map(|attr| Attribute {
                        name: attr.name.borrow(),
                        value: attr.value.as_str(),
                    })
                    .collect();

                if inside_target_element {
                    let mut start_element = WriterXmlEvent::start_element(name.local_name.as_str());

                    for attr in attributes {
                        start_element = start_element.attr(attr.name.local_name, attr.value);
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

                            // Verifica o tipo de keyword (${keyword}, ${#keyword}, ${/keyword})
                            if accumulated_text.contains("${#") {
                                // Handle block start
                                // Omit any writing until block is complete
                            } else if accumulated_text.contains("${/") {
                                // Handle block end
                                // Process the accumulated block
                            } else {
                                // Handle simple keyword replacement
                                // Perform any necessary replacement and write the text
                                writer.write(WriterXmlEvent::characters(&accumulated_text))?;
                            }

                            accumulated_text.clear(); // Clear after processing
                        }
                    } else {
                        // Se não estamos dentro de uma keyword, podemos escrever o texto diretamente
                        writer.write(WriterXmlEvent::characters(&accumulated_text))?;
                        accumulated_text.clear(); // Clear after writing
                    }
                } else {
                    writer.write(WriterXmlEvent::characters(&text))?;
                }
            }
            XmlEvent::EndElement { name } => {
                if inside_target_element {
                    writer.write(WriterXmlEvent::end_element())?;
                }

                // Quando encontrarmos o fechamento da tag <p> ou <tr>, saímos do modo de captura
                if inside_target_element && name.local_name == element_name {
                    inside_target_element = false;
                    // Encerra o empréstimo do writer antes de acessar `output`
                    drop(writer);

                    // Agora que o buffer foi escrito no output, podemos processar o output
                    let content = String::from_utf8(output.clone())?;
                    println!("Conteúdo do elemento: {}", content);

                    output.clear();
                    // Recriar o writer para continuar escrevendo no mesmo `output`
                    writer = EmitterConfig::new()
                        .perform_indent(true)
                        .create_writer(&mut output);
                }

                // Remover o nome do elemento da pilha do caminho
                if let Some(last) = path_stack.pop_back() {
                    if last == "document" {
                        println!("Conteúdo do last: {}", last);
                        break; // Sai do loop quando encontrar o fechamento do <document>
                    }

                    if last == "body" {
                        inside_body = false;
                    }
                }
            }

            _ => {
                // Captura e imprime o tipo do evento não tratado especificamente
                println!("Evento não tratado: {:?}", event);
            }
        }
    }

    Ok(())
}
