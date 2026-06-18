/// DOCX 文件解析器
/// .docx 本质是一个 ZIP 包，正文在 word/document.xml 中

use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

/// 从 DOCX 文件中提取纯文本
pub fn extract_text(path: &str) -> Result<String, String> {
    let file = File::open(path)
        .map_err(|e| format!("无法打开文件: {}", e))?;

    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("无法解析 ZIP/DOCX: {}", e))?;

    // 查找 word/document.xml
    let mut doc_xml = String::new();
    let mut found = false;
    
    // 尝试按优先级读取
    let paths = ["word/document.xml", "word/document2.xml", "document.xml"];
    for p in &paths {
        if let Ok(mut entry) = archive.by_name(p) {
            entry.read_to_string(&mut doc_xml)
                .map_err(|e| format!("读取 {} 失败: {}", p, e))?;
            found = true;
            break;
        }
    }
    
    if !found {
        return Err("未找到 word/document.xml".into());
    }

    // 从 XML 中提取 <w:t> 标签内的文本
    let text = extract_text_from_docx_xml(&doc_xml);
    Ok(text)
}

/// 从 DOCX 的 XML 中提取 `<w:t>` 标签内的文本
fn extract_text_from_docx_xml(xml: &str) -> String {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut in_wt = false;
    let mut result = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name_bytes = e.name().as_ref().to_vec();
                let name = String::from_utf8_lossy(&name_bytes);
                match name.as_ref() {
                    "w:p" | "w:pPr" => {
                        if !result.is_empty() && !result.ends_with('\n') {
                            result.push('\n');
                        }
                    }
                    "w:t" => {
                        in_wt = true;
                    }
                    "w:br" => {
                        result.push('\n');
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => {
                if in_wt {
                    let text = e.unescape().unwrap_or_default();
                    result.push_str(&text);
                }
            }
            Ok(Event::End(ref e)) => {
                let name_bytes = e.name().as_ref().to_vec();
                let name = String::from_utf8_lossy(&name_bytes);
                if name.as_ref() == "w:t" {
                    in_wt = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                eprintln!("[warn] XML 解析警告: {}", e);
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    // 清理多余空行
    let trimmed: String = result
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    trimmed
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::FileOptions;

    fn create_test_docx() -> String {
        // 创建一个最小的 .docx 文件（ZIP 包）
        let dir = std::env::temp_dir().join("docx_test");
        std::fs::create_dir_all(&dir).ok();
        
        let docx_path = dir.join("test.docx");
        
        // 如果已存在则直接返回
        if docx_path.exists() {
            return docx_path.to_str().unwrap().to_string();
        }

        // 创建 ZIP 文件
        let file = File::create(&docx_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);

        // 添加 mimetype (必须第一个条目)
        let opts: zip::write::FileOptions<'_, ()> = Default::default();
        zip.start_file("mimetype", opts).ok();
        zip.write_all(b"application/vnd.openxmlformats-officedocument.wordprocessingml.document").ok();

        // word/document.xml
        let doc_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>测试文档标题</w:t></w:r></w:p>
    <w:p><w:r><w:t>联系人：张三</w:t></w:r></w:p>
    <w:p><w:r><w:t>电话：13800138000</w:t></w:r></w:p>
    <w:p><w:r><w:t>身份证号：110101199003074477</w:t></w:r></w:p>
    <w:p><w:r><w:t>金额：¥500,000.00</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
        let opts: zip::write::FileOptions<'_, ()> = Default::default();
        zip.start_file("word/document.xml", opts).ok();
        zip.write_all(doc_xml.as_bytes()).ok();

        // [Content_Types].xml
        let ct = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#;
        let opts: zip::write::FileOptions<'_, ()> = Default::default();
        zip.start_file("[Content_Types].xml", opts).ok();
        zip.write_all(ct.as_bytes()).ok();

        zip.finish().unwrap();
        docx_path.to_str().unwrap().to_string()
    }

    #[test]
    fn test_docx_extract() {
        let path = create_test_docx();
        let text = extract_text(&path).unwrap();
        println!("提取的文本:\n{}", text);
        assert!(text.contains("张三"));
        assert!(text.contains("13800138000"));
    }
}
