use chrono::{Datelike, NaiveDate};
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Error};
use std::path::{Path, PathBuf};

use sys_locale::get_locale;

// Captura o idioma do sistema operacional ou usa um padrão
pub static LOCALE: Lazy<String> = Lazy::new(|| set_locale());

// Função para setar o LOCALE
pub fn set_locale() -> String {
    let system_locale = get_locale().unwrap_or_else(|| "pt_BR".to_string());
    if TRANSLATIONS.contains_key(&system_locale) {
        system_locale
    } else {
        "pt_BR".to_string()
    }
}

// Lazy static variable to load translations once
pub static TRANSLATIONS: Lazy<HashMap<String, Value>> = Lazy::new(|| {
    load_translations(Some(Path::new("resources/translations.json"))).unwrap_or_else(|_| {
        eprintln!("Failed to load translations.");
        HashMap::new()
    })
});
// Carrega as traduções no início do programa e as mantém disponíveis globalmente.

/// Carrega as traduções de um arquivo JSON e retorna como um HashMap.
pub fn load_translations<P: AsRef<Path>>(
    path: Option<P>,
) -> Result<HashMap<String, Value>, Box<dyn std::error::Error>> {
    let default_path = PathBuf::from("resources/translations.json");
    let path = path
        .map(|p| p.as_ref().to_path_buf())
        .unwrap_or(default_path);

    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let translations = serde_json::from_reader(reader)?;
    Ok(translations)
}

pub fn format_date_with_locale(date_str: &str, format: &str, locale: &str) -> String {
    let possible_formats = ["%Y-%m-%d", "%Y/%m/%d", "%d-%m-%Y", "%d/%m/%Y"];

    for &date_format in &possible_formats {
        if let Ok(date) = NaiveDate::parse_from_str(date_str, date_format) {
            match format {
                "short" => return date.format("%d/%m/%Y").to_string(),
                "long" => {
                    if let Some(locale_translations) = TRANSLATIONS.get(locale) {
                        if let Some(months) = locale_translations.get("months") {
                            if let Some(month_name) = months.get(&date.month().to_string()) {
                                let preposition = locale_translations
                                    .get("prepositions")
                                    .and_then(|p| p.get("of"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("de");

                                return format!(
                                    "{} {} {} {} {}",
                                    date.day(),
                                    preposition,
                                    month_name.as_str().unwrap_or(""),
                                    preposition,
                                    date.year()
                                );
                            }
                        }
                    }
                    return date.format("%d/%m/%Y").to_string(); // fallback
                }
                _ => return date.format("%d/%m/%Y").to_string(),
            }
        }
    }

    date_str.to_string() // Retorna a string original se o parsing falhar
}

/// Formata uma data baseada no locale e nas traduções fornecidas.
/* pub fn format_date_with_locale(
    date_str: &str,
    format: &str,
    locale: &str,
    translations: &HashMap<String, Value>,
) -> String {
    let possible_formats = ["%Y-%m-%d", "%Y/%m/%d", "%d-%m-%Y", "%d/%m/%Y"];

    for &date_format in &possible_formats {
        if let Ok(date) = NaiveDate::parse_from_str(date_str, date_format) {
            match format {
                "short" => return date.format("%d/%m/%Y").to_string(),
                "long" => {
                    if let Some(locale_translations) = translations.get(locale) {
                        if let Some(months) = locale_translations.get("months") {
                            if let Some(month_name) = months.get(&date.month().to_string()) {
                                return format!(
                                    "{} de {} de {}",
                                    date.day(),
                                    month_name.as_str().unwrap_or(""),
                                    date.year()
                                );
                            }
                        }
                    }
                    return date.format("%d/%m/%Y").to_string(); // fallback
                }
                _ => return date.format("%d/%m/%Y").to_string(),
            }
        }
    }

    date_str.to_string() // Retorna a string original se o parsing falhar
}
 */
/*
pub fn format_date_with_locale(
    date_str: &str,
    format: &str,
    locale: &str,
    translations: &HashMap<String, Value>,
) -> String {
    let possible_formats = ["%Y-%m-%d", "%Y/%m/%d", "%d-%m-%Y", "%d/%m/%Y"];

    for &date_format in &possible_formats {
        if let Ok(date) = NaiveDate::parse_from_str(date_str, date_format) {
            match format {
                "short" => return date.format("%d/%m/%Y").to_string(),
                "long" => {
                    let month = date.month().to_string();
                    if let Some(locale_translations) = translations.get(locale) {
                        if let Some(months) = locale_translations.get("months") {
                            if let Some(month_name) = months.get(&month) {
                                return format!(
                                    "{} de {} de {}",
                                    date.day(),
                                    month_name.as_str().unwrap_or(""),
                                    date.year()
                                );
                            }
                        }
                    }
                    return date.format("%d/%m/%Y").to_string(); // fallback
                }
                _ => return date.format("%d/%m/%Y").to_string(),
            }
        }
    }

    date_str.to_string() // Retorna a string original se o parsing falhar
}
 */

#[cfg(test)]
mod tests_load_translations {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_load_translations_with_valid_path() {
        // Cria um diretório temporário para o teste
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("translations.json");

        // Cria um arquivo de exemplo de traduções JSON
        let mut file = File::create(&file_path).unwrap();
        writeln!(
            file,
            r#"{{
                "months": {{
                    "January": "Janeiro",
                    "February": "Fevereiro",
                    "March": "Março"
                }},
                "boolean": {{
                    "true": "Verdadeiro",
                    "false": "Falso"
                }}
            }}"#
        )
        .unwrap();

        // Carrega as traduções do arquivo criado
        let translations = load_translations(Some(file_path)).unwrap();

        // Verifica se as traduções foram carregadas corretamente
        assert_eq!(translations["months"]["January"], "Janeiro");
        assert_eq!(translations["boolean"]["true"], "Verdadeiro");

        // Limpa o diretório temporário
        dir.close().unwrap();
    }

    #[test]
    fn test_load_translations_with_default_path() {
        // Cria um arquivo de exemplo de traduções JSON no caminho padrão
        let dir = tempdir().unwrap();
        let default_path = dir.path().join("resources/translations.json");

        std::fs::create_dir_all(default_path.parent().unwrap()).unwrap();
        let mut file = File::create(&default_path).unwrap();
        writeln!(
            file,
            r#"{{
                "months": {{
                    "April": "Abril",
                    "May": "Maio",
                    "June": "Junho"
                }},
                "boolean": {{
                    "true": "Verdadeiro",
                    "false": "Falso"
                }}
            }}"#
        )
        .unwrap();

        // Define o diretório padrão como o local para procurar traduções
        let translations = load_translations(None as Option<&str>).unwrap();

        // Verifica se as traduções foram carregadas corretamente
        assert_eq!(translations["months"]["April"], "Abril");
        assert_eq!(translations["boolean"]["false"], "Falso");

        // Limpa o diretório temporário
        dir.close().unwrap();
    }

    #[test]
    #[should_panic(expected = "No such file or directory")]
    fn test_load_translations_with_invalid_path() {
        // Tenta carregar traduções de um caminho que não existe
        let _translations = load_translations(Some("invalid/path/to/translations.json")).unwrap();
    }
}
