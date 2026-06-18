/// PDF 文件解析器

/// 从 PDF 文件中提取纯文本
pub fn extract_text(path: &str) -> Result<String, String> {
    let bytes = std::fs::read(path)
        .map_err(|e| format!("读取 PDF 失败: {}", e))?;

    let text = pdf_extract::extract_text_from_mem(&bytes)
        .map_err(|e| format!("PDF 文本提取失败: {}", e))?;

    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_extract_no_file() {
        let result = extract_text("/nonexistent/test.pdf");
        assert!(result.is_err());
    }
}
