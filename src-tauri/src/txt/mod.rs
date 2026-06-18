/// TXT 文本文件读取模块
/// 将纯文本内容包装为与 Excel 兼容的表格结构
/// （每段为一行，单列"正文内容"）

use crate::excel::{ColumnInfo, ExcelData};
use std::fs;

/// 读取 TXT 文件，返回 ExcelData 兼容结构
pub fn read_txt(path: &str) -> Result<ExcelData, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("读取 TXT 文件失败: {}", e))?;

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
                // 取每段前50字符作为样本
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

    Ok(ExcelData {
        sheet_name: "正文".into(),
        columns: vec![column],
        rows,
        total_rows: rows.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_read_txt() {
        let mut tmpfile = tempfile::NamedTempFile::new().unwrap();
        write!(tmpfile, "第一段内容\n\n第二段内容\n\n第三段内容").unwrap();
        let result = read_txt(tmpfile.path().to_str().unwrap()).unwrap();
        assert_eq!(result.total_rows, 3);
        assert_eq!(result.columns[0].name, "正文内容");
    }
}
