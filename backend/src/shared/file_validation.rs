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
        "Validated file: {filename} ({file_type:?}, {} bytes)",
        content.len()
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
                _ => None,
            }
        })
        .ok_or_else(|| {
            let reason = format!("Unsupported file extension: {filename}");
            tracing::warn!("File rejected: {filename} - {reason}");
            AppError::FileError(reason)
        })?;

    // Validate magic bytes
    match extension {
        FileType::Pdf => validate_pdf_magic(content),
        FileType::Markdown => Ok(()), // No magic bytes for MD — trust extension
        FileType::Docx => validate_docx_magic(content),
        FileType::Zip => validate_zip_magic(content),
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
    tracing::debug!("ZIP magic bytes validated: {} bytes", content.len());
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
}
