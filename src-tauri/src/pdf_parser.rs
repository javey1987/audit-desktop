/// PDF 文件解析器
/// 使用系统命令 pdftotext（来自 poppler-utils）提取文本
/// 避免引入繁重的 Rust PDF 解析库

use std::process::Command;

/// 从 PDF 文件中提取纯文本
pub fn extract_text(path: &str) -> Result<String, String> {
    let output = Command::new("pdftotext")
        .arg(path)
        .arg("-")  // 输出到 stdout
        .output()
        .map_err(|e| {
            format!(
                "调用 pdftotext 失败: {}。请安装 poppler-utils: sudo apt install poppler-utils",
                e
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("pdftotext 提取失败: {}", stderr));
    }

    let text = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_missing_file() {
        let result = extract_text("/nonexistent/test.pdf");
        assert!(result.is_err());
    }
}
