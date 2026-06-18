/// TXT 文本文件读取模块
/// 将纯文本内容包装为与 Excel 兼容的表格结构
/// （每段为一行，单列"正文内容"）

use crate::excel::{ColumnInfo, ExcelData};
use std::fs;

/// 从内容字符串构造 ExcelData
pub fn content_to_excel(content: &str, sheet_name: &str) -> Result<ExcelData, String> {
    if content.trim().is_empty() {
        return Err("文件内容为空".into());
    }

    // 按段落分割（空行分隔），过滤空行
    let paragraphs: Vec<&str> = content
        .split("\n\n")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let column = ColumnInfo {
        name: "正文内容".into(),
        sample_values: {
            let mut samples = Vec::new();
            for p in &paragraphs {
                let preview: String = p.chars().take(50).collect();
                if !preview.is_empty() && samples.len() < 5 {
                    samples.push(preview);
                }
            }
            samples
        },
    };

    let rows: Vec<Vec<String>> = paragraphs
        .iter()
        .map(|p| vec![p.to_string()])
        .collect();

    let total_rows = rows.len();
    Ok(ExcelData {
        sheet_name: sheet_name.into(),
        columns: vec![column],
        rows,
        total_rows,
    })
}

/// 读取 TXT 文件，返回 ExcelData 兼容结构
pub fn read_txt(path: &str) -> Result<ExcelData, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("读取 TXT 文件失败: {}", e))?;
    content_to_excel(&content, "正文")
}

/// 从内存中的字符串创建 ExcelData（用于 PDF/DOCX 等已解析的文本）
pub fn read_txt_from_str(content: &str) -> Result<ExcelData, String> {
    content_to_excel(content, "解析内容")
}
