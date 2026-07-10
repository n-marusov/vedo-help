use crate::shared::error::AppError;
use crate::shared::types::FileType;

/// Maximum allowed file size in bytes (50 MB).
pub const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024;

/// Magic bytes for ZIP-based formats (DOCX, ZIP, XLSX, etc.): `PK\x03\x04`
const ZIP_MAGIC: &[u8] = b"PK\x03\x04";

/// Validate an uploaded file: check MIME type, magic bytes, and size.
///
/// Returns the detected `FileType` on success.
pub fn validate_file(content: &[u8], filename: &str) -> Result<FileType, AppError> {
    if content.is_empty() {
        return Err(AppError::FileError("Empty file".to_string()));
    }

    if (content.len() as u64) > MAX_FILE_SIZE {
        return Err(AppError::FileError(format!(
            "File exceeds maximum size of {} MB",
            MAX_FILE_SIZE / (1024 * 1024)
        )));
    }

    let file_type = detect_file_type(filename, content)?;

    tracing::info!(
        component = "file_validation",
        file_name = %filename,
        file_type = %format!("{:?}", file_type),
        file_size = content.len(),
        "file.validated"
    );

    Ok(file_type)
}

/// Detect the file type from extension and magic bytes.
fn detect_file_type(filename: &str, content: &[u8]) -> Result<FileType, AppError> {
    let extension = filename
        .rsplit('.')
        .next()
        .and_then(|ext| {
            let ext = ext.to_lowercase();
            match ext.as_str() {
                "pdf" => Some(FileType::Pdf),
                "md" | "markdown" => Some(FileType::Markdown),
                "docx" => Some(FileType::Docx),
                "zip" => Some(FileType::Zip),
                "csv" => Some(FileType::Csv),
                "json" => Some(FileType::Json),
                "html" | "htm" => Some(FileType::Html),
                _ => None,
            }
        })
        .ok_or_else(|| {
            let reason = format!("Unsupported file extension: {filename}");
            tracing::warn!(component = "file_validation", file_name = %filename, reason = %reason, "file.rejected");
            AppError::FileError(reason)
        })?;

    // Validate magic bytes
    match extension {
        FileType::Pdf => validate_pdf_magic(content),
        FileType::Markdown => Ok(()), // No magic bytes for MD — trust extension
        FileType::Docx => validate_docx_magic(content),
        FileType::Zip => validate_zip_magic(content),
        FileType::Csv => Ok(()), // No magic bytes for CSV — trust extension
        FileType::Json => validate_json_magic(content),
        FileType::Html => validate_html_magic(content),
    }?;

    Ok(extension)
}

/// Check PDF magic bytes: `%PDF` at offset 0.
fn validate_pdf_magic(content: &[u8]) -> Result<(), AppError> {
    if content.len() < 4 || &content[0..4] != b"%PDF" {
        return Err(AppError::FileError(
            "Invalid PDF file: missing PDF header".to_string(),
        ));
    }
    Ok(())
}

/// Check DOCX magic bytes: `PK\x03\x04` (ZIP header).
fn validate_docx_magic(content: &[u8]) -> Result<(), AppError> {
    if content.len() < 4 || &content[0..4] != ZIP_MAGIC {
        return Err(AppError::FileError(
            "Invalid DOCX file: missing ZIP header".to_string(),
        ));
    }
    Ok(())
}

/// Check ZIP magic bytes: `PK\x03\x04` (ZIP header).
pub fn validate_zip_magic(content: &[u8]) -> Result<(), AppError> {
    if content.len() < 4 || &content[0..4] != ZIP_MAGIC {
        return Err(AppError::FileError(
            "Invalid ZIP file: missing ZIP header".to_string(),
        ));
    }
    tracing::debug!(
        component = "file_validation",
        file_size = content.len(),
        "zip.magic_bytes_validated"
    );
    Ok(())
}

/// Check JSON magic bytes: must start with `{` or `[` (after optional BOM).
fn validate_json_magic(content: &[u8]) -> Result<(), AppError> {
    let start = if content.len() > 3 && content[0..3] == [0xEF, 0xBB, 0xBF] {
        3 // skip UTF-8 BOM
    } else {
        0
    };
    if content.len() <= start {
        return Err(AppError::FileError(
            "Invalid JSON file: empty content".to_string(),
        ));
    }
    let trimmed = content[start..]
        .iter()
        .copied()
        .skip_while(|&b| b == b' ' || b == b'\n' || b == b'\r' || b == b'\t')
        .collect::<Vec<_>>();
    if trimmed.is_empty() || (trimmed[0] != b'{' && trimmed[0] != b'[') {
        return Err(AppError::FileError(
            "Invalid JSON file: must start with '{' or '['".to_string(),
        ));
    }
    Ok(())
}

/// Check HTML magic bytes: must start with `<` (after optional BOM/whitespace).
fn validate_html_magic(content: &[u8]) -> Result<(), AppError> {
    let start = if content.len() > 3 && content[0..3] == [0xEF, 0xBB, 0xBF] {
        3
    } else {
        0
    };
    if content.len() <= start {
        return Err(AppError::FileError(
            "Invalid HTML file: empty content".to_string(),
        ));
    }
    // Skip leading whitespace before checking `<`
    if !content[start..].contains(&b'<') {
        return Err(AppError::FileError(
            "Invalid HTML file: must contain '<' character".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_pdf_valid() {
        let content = b"%PDF-1.4\n...";
        assert!(validate_pdf_magic(content).is_ok());
    }

    #[test]
    fn test_validate_pdf_invalid() {
        let content = b"Not a PDF file";
        assert!(validate_pdf_magic(content).is_err());
    }

    #[test]
    fn test_validate_docx_valid() {
        let content = b"PK\x03\x04...";
        assert!(validate_docx_magic(content).is_ok());
    }

    #[test]
    fn test_validate_docx_invalid() {
        let content = b"Not a DOCX file";
        assert!(validate_docx_magic(content).is_err());
    }

    #[test]
    fn test_validate_file_by_extension() {
        let pdf_content = b"%PDF-1.4 content";
        let result = validate_file(pdf_content, "test.pdf");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), FileType::Pdf);

        let md_content = b"# Markdown content";
        let result = validate_file(md_content, "readme.md");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), FileType::Markdown);

        let docx_content = b"PK\x03\x04 content";
        let result = validate_file(docx_content, "test.docx");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), FileType::Docx);
    }

    #[test]
    fn test_validate_file_unsupported_extension() {
        let content = b"some content";
        let result = validate_file(content, "test.txt");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }

    #[test]
    fn test_validate_file_empty() {
        let result = validate_file(b"", "test.pdf");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_file_too_large() {
        let content = vec![0u8; (MAX_FILE_SIZE + 1) as usize];
        let result = validate_file(&content, "test.pdf");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("exceeds maximum size"));
    }

    #[test]
    fn test_validate_zip_valid() {
        let content = b"PK\x03\x04...";
        assert!(validate_zip_magic(content).is_ok());
        let result = validate_file(content, "archive.zip");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), FileType::Zip);
    }

    #[test]
    fn test_validate_zip_invalid_magic() {
        let content = b"Not a ZIP file";
        assert!(validate_zip_magic(content).is_err());
        let result = validate_file(content, "archive.zip");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("missing ZIP header"));
    }

    #[test]
    fn test_validate_zip_empty() {
        let result = validate_zip_magic(b"");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_zip_extension() {
        let content = b"PK\x03\x04 content";
        let result = validate_file(content, "data.zip");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), FileType::Zip);
    }

    // ── CSV tests ──

    #[test]
    fn test_validate_csv_valid() {
        let content = b"name,age,city\nAlice,30,NYC\nBob,25,LA";
        let result = validate_file(content, "data.csv");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), FileType::Csv);
    }

    #[test]
    fn test_validate_csv_empty() {
        let result = validate_file(b"", "data.csv");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Empty file"));
    }

    // ── JSON tests ──

    #[test]
    fn test_validate_json_valid_object() {
        let content = b"{\"key\": \"value\"}";
        let result = validate_file(content, "data.json");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), FileType::Json);
    }

    #[test]
    fn test_validate_json_valid_array() {
        let content = b"[1, 2, 3]";
        let result = validate_file(content, "data.json");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), FileType::Json);
    }

    #[test]
    fn test_validate_json_invalid_start() {
        let content = b"Not JSON content";
        let result = validate_file(content, "data.json");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must start with"));
    }

    #[test]
    fn test_validate_json_with_bom() {
        let content = [0xEF, 0xBB, 0xBF, b'{', b'"', b'a', b'"', b':', b'1', b'}'];
        let result = validate_file(&content, "data.json");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), FileType::Json);
    }

    #[test]
    fn test_validate_json_empty() {
        let result = validate_file(b"", "data.json");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Empty file"));
    }

    // ── HTML tests ──

    #[test]
    fn test_validate_html_valid() {
        let content = b"<html><body>Hello</body></html>";
        let result = validate_file(content, "page.html");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), FileType::Html);
    }

    #[test]
    fn test_validate_html_short() {
        let content = b"<p>Hi</p>";
        let result = validate_file(content, "page.html");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), FileType::Html);
    }

    #[test]
    fn test_validate_html_invalid_no_tag() {
        let content = b"Just plain text without any HTML tags";
        let result = validate_file(content, "page.html");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must contain"));
    }

    #[test]
    fn test_validate_html_empty() {
        let result = validate_file(b"", "page.html");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_html_htm_extension() {
        let content = b"<html>\n<body>\n<p>Test</p>\n</body>\n</html>";
        let result = validate_file(content, "page.htm");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), FileType::Html);
    }

    // ── MIME types ──

    #[test]
    fn test_csv_mime_type() {
        assert_eq!(FileType::Csv.mime_type(), "text/csv");
    }

    #[test]
    fn test_json_mime_type() {
        assert_eq!(FileType::Json.mime_type(), "application/json");
    }

    #[test]
    fn test_html_mime_type() {
        assert_eq!(FileType::Html.mime_type(), "text/html");
    }

    #[test]
    fn test_from_extension_csv() {
        assert_eq!(FileType::from_extension("data.csv"), Some(FileType::Csv));
    }

    #[test]
    fn test_from_extension_json() {
        assert_eq!(FileType::from_extension("data.json"), Some(FileType::Json));
    }

    #[test]
    fn test_from_extension_html() {
        assert_eq!(FileType::from_extension("page.html"), Some(FileType::Html));
        assert_eq!(FileType::from_extension("page.htm"), Some(FileType::Html));
    }
}
