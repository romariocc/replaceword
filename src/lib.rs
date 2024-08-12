mod read_paragraphs;
mod write_paragraphs;

use serde_json::Value;
use std::path::Path;

use std::path::PathBuf;

pub fn replace(
    input_path: &str,
    data: &Value,
    output_path: &str,
    output_filename: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if !Path::new(output_path).exists() {
        std::fs::create_dir_all(output_path)?;
    }

    let output_path = format!("{}/{}.doc", output_path, output_filename);

    let output_file_path =
        PathBuf::from(output_path.clone()).join(format!("{}.doc", output_path.clone()));

    match write_paragraphs::modify_docx_paragraphs(input_path, &output_path, data) {
        Ok(_) => println!("Novo documento criado com sucesso!"),
        Err(e) => eprintln!("Erro ao criar novo documento: {}", e),
    }

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
        let output_filename = "documento_final";

        let result = replace(input_path, &data, output_path, output_filename);

        println!("Resultado do replace: {:?}", result);
        assert!(result.is_ok());

        let output_file_path = format!("{}/{}.doc", output_path, output_filename);
        let metadata = fs::metadata(&output_file_path);
        println!("Metadata do arquivo gerado: {:?}", metadata);
        assert!(metadata.is_ok());
    }
}
