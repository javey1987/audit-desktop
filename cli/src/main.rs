/// 脱敏引擎 CLI 原型
///
/// 用法:
///   desense scan <file>              — 扫描文件中的敏感数据并脱敏
///   desense scan <file> --dict <yaml> — 带自定义词表扫描
///   desense restore <file> <map>     — 从脱敏文件+映射表还原
///   desense test                     — 运行内建测试
///
/// 输出:
///   scan: 打印脱敏结果 + 映射表 JSON 文件
///   restore: 打印还原后的原文

use clap::{Parser, Subcommand};
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

mod scanner;
mod tokenizer;
mod docx_parser;
mod pdf_parser;

use tokenizer::Tokenizer;

// ---- 自定义词典结构 ----

/// YAML 格式的敏感词表
#[derive(serde::Deserialize)]
struct CustomDict {
    persons: Option<Vec<String>>,
    orgs: Option<Vec<String>>,
    projects: Option<Vec<String>>,
    systems: Option<Vec<String>>,
    partners: Option<Vec<String>>,
    #[serde(flatten)]
    other: HashMap<String, serde_yaml::Value>,
}

// ---- CLI ----

#[derive(Parser)]
#[command(name = "desense", version = "0.1.0", about = "脱敏引擎 CLI 原型")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 扫描文件中的敏感数据并脱敏
    Scan {
        /// 要扫描的文件路径
        file: PathBuf,
        /// 自定义敏感词表（YAML 格式）
        #[arg(long, short)]
        dict: Option<PathBuf>,
        /// 输出映射表到文件
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
        /// 输出脱敏文本到文件
        #[arg(long)]
        out_text: Option<PathBuf>,
    },
    /// 从脱敏文件+映射表还原
    Restore {
        /// 脱敏后的文本文件
        file: PathBuf,
        /// 映射表 JSON 文件
        map: PathBuf,
    },
    /// 运行内建测试
    Test {
        /// 测试文本（可选，不提供则用默认文本）
        text: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Scan { file, dict, output, out_text } => {
            cmd_scan(file, dict.as_ref(), output.as_ref(), out_text.as_ref());
        }
        Commands::Restore { file, map } => {
            cmd_restore(file, map);
        }
        Commands::Test { text } => {
            cmd_test(text.as_deref());
        }
    }
}

fn cmd_scan(
    file: &PathBuf,
    dict: Option<&PathBuf>,
    output: Option<&PathBuf>,
    out_text: Option<&PathBuf>,
) {
    // 根据文件类型自动选择解析方式
    let content = match read_file_content(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("❌ 读取文件失败: {}", e);
            std::process::exit(1);
        }
    };

    // 初始化标记器
    let mut tokenizer = Tokenizer::new();

    // 加载自定义词表
    if let Some(dict_path) = dict {
        match load_custom_dict(dict_path) {
            Ok(dict_map) => {
                let total: usize = dict_map.values().map(|v| v.len()).sum();
                println!("📖 已加载自定义词表，共 {} 条", total);
                tokenizer.load_custom_dict(dict_map);
            }
            Err(e) => {
                eprintln!("⚠️  加载自定义词表失败: {}（跳过）", e);
            }
        }
    }

    // 执行标记化
    let (sanitized, map) = tokenizer.tokenize(&content);
    let stats = map.stat();

    // 输出结果
    println!("\n═══════════════════════════════════════");
    println!(" 📋 脱敏结果");
    println!("═══════════════════════════════════════");
    println!("{}", sanitized);
    println!("\n═══════════════════════════════════════");

    // 统计
    println!("\n📊 脱敏统计:");
    let total = map.entries.len();
    if total == 0 {
        println!("   ✅ 未检测到敏感数据");
    } else {
        println!("   共 {} 处敏感数据：", total);
        for (entity_type, count) in &stats {
            let label = match entity_type.as_str() {
                "PERSON" => "人名",
                "ORG" => "公司/组织",
                "PHONE" => "手机号",
                "ID_CARD" => "身份证号",
                "BANK" => "银行卡号",
                "EMAIL" => "邮箱",
                "ADDR" => "地址",
                "URL" => "URL",
                "IP" => "IP地址",
                "MONEY" => "金额",
                "PROJECT" => "项目名",
                s => s,
            };
            println!("   • {}({}) : {} 处", entity_type, label, count);
        }

        // 映射表详情
        println!("\n📌 映射表:");
        for entry in &map.entries {
            println!("   {}  ←  {}  [{}]", entry.placeholder, entry.original, entry.entity_label);
        }
    }

    // 还原验证
    let restored = map.restore(&sanitized);
    if restored == content {
        println!("\n✅ 还原验证通过 — 完整可逆");
    } else {
        println!("\n⚠️  还原后有差异（可能是标记格式问题）");
    }

    // 输出文件
    if let Some(o) = output {
        let json = serde_json::to_string_pretty(&map).unwrap();
        fs::write(o, json).unwrap_or_else(|e| eprintln!("⚠️  写入映射表文件失败: {}", e));
        println!("\n💾 映射表已保存: {}", o.display());
    }

    if let Some(o) = out_text {
        fs::write(o, &sanitized).unwrap_or_else(|e| eprintln!("⚠️  写入脱敏文本失败: {}", e));
        println!("💾 脱敏文本已保存: {}", o.display());
    }
}

fn cmd_restore(file: &PathBuf, map_file: &PathBuf) {
    // 读取脱敏文本
    let sanitized = match fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("❌ 读取脱敏文件失败: {}", e);
            std::process::exit(1);
        }
    };

    // 读取映射表
    let map: tokenizer::TokenMap = match fs::read_to_string(map_file) {
        Ok(s) => match serde_json::from_str(&s) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("❌ 解析映射表 JSON 失败: {}", e);
                std::process::exit(1);
            }
        },
        Err(e) => {
            eprintln!("❌ 读取映射表文件失败: {}", e);
            std::process::exit(1);
        }
    };

    // 还原
    let restored = map.restore(&sanitized);

    println!("\n═══════════════════════════════════════");
    println!(" 🔄 还原结果");
    println!("═══════════════════════════════════════");
    println!("{}", restored);
    println!("\n═══════════════════════════════════════");
}

/// 判断是否是图片文件
fn is_image_file(lower: &str) -> bool {
    lower.ends_with(".png") || lower.ends_with(".jpg")
    || lower.ends_with(".jpeg") || lower.ends_with(".bmp")
    || lower.ends_with(".tiff") || lower.ends_with(".tif")
    || lower.ends_with(".webp")
}

/// 将 .doc 文件转换为纯文本
/// 优先使用 catdoc（轻量），回退到 libreoffice
fn convert_doc_to_text(path: &str) -> Result<String, String> {
    // 方案1：catdoc（推荐，轻量）
    let output = std::process::Command::new("catdoc")
        .arg(path)
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout).to_string();
            if !text.trim().is_empty() {
                return Ok(text);
            }
        }
    }

    // 方案2：antiword
    let output = std::process::Command::new("antiword")
        .arg(path)
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout).to_string();
            if !text.trim().is_empty() {
                return Ok(text);
            }
        }
    }

    // 方案3：libreoffice（最重但最可靠）
    convert_via_libreoffice(path, "doc")
}

/// 将 .wps 文件转换为纯文本
fn convert_wps_to_text(path: &str) -> Result<String, String> {
    convert_via_libreoffice(path, "wps")
}

/// 使用 libreoffice 将文件转换为纯文本
fn convert_via_libreoffice(path: &str, _fmt: &str) -> Result<String, String> {
    let out_dir = std::env::temp_dir().join("desense_tmp");
    std::fs::create_dir_all(&out_dir).ok();

    let status = std::process::Command::new("libreoffice")
        .args(["--headless", "--convert-to", "txt:Text",
               "--outdir", out_dir.to_str().unwrap_or("/tmp"),
               path])
        .status()
        .map_err(|e| {
            format!(
                "调用 libreoffice 失败: {}。请安装: sudo apt install libreoffice-writer catdoc",
                e
            )
        })?;

    if !status.success() {
        return Err(format!("libreoffice 转换失败（exit code: {:?}）", status.code()));
    }

    // 查找输出的 .txt 文件
    let base = std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let out_path = out_dir.join(format!("{}.txt", base));

    let text = std::fs::read_to_string(&out_path)
        .map_err(|e| format!("读取转换结果失败: {}", e))?;

    // 清理临时文件
    std::fs::remove_file(&out_path).ok();

    Ok(text)
}

/// 调用 tesseract OCR 识别图片中的文字
fn ocr_image(path: &str) -> Result<String, String> {
    let output = std::process::Command::new("tesseract")
        .arg(path)
        .arg("stdout")
        .arg("-l")
        .arg("chi_sim+eng")  // 中文简体 + 英文
        .output()
        .map_err(|e| {
            format!(
                "调用 tesseract 失败: {}。请安装: sudo apt install tesseract-ocr tesseract-ocr-chi-sim",
                e
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("OCR 识别失败: {}", stderr));
    }

    let text = String::from_utf8_lossy(&output.stdout).to_string();
    if text.trim().is_empty() {
        return Err("OCR 未识别到任何文字".into());
    }
    Ok(text)
}

fn cmd_test(text: Option<&str>) {
    let test_text = text.unwrap_or(
        r#"==== 审计测试数据 ====

客户信息：
- 姓名：张三
- 手机：13800138000
- 邮箱：zhangsan@company.com.cn
- 身份证：110101199003074477
- 银行卡号：6222021234567890123
- 地址：北京市海淀区中关村南大街5号院2号楼

供应商：深圳华为技术有限公司（项目负责人：李四，电话：13912345678）
项目成本：¥5,000,000.00，已付款80%，合同金额500万元。
内网地址：http://192.168.1.100:8080/project/dashboard

内部系统：宙斯计划（SAP-ERP v3.0），部署在 git.internal.com:22

其他联系人：王五（13812345678）、赵六（zhaoliu@163.com）、
郑七的身份证号是 32010219950812777X

另有一笔异常交易：从 6225768901234567 汇出￥888,888.88 
到开户行：中国工商银行北京分行
"#
    );

    println!("═══════════════════════════════════════");
    println!(" 🧪 脱敏引擎测试");
    println!("═══════════════════════════════════════");

    let mut tokenizer = Tokenizer::new();
    let (sanitized, map) = tokenizer.tokenize(test_text);
    let stats = map.stat();

    println!("\n📥 原文 ({})", test_text.len());
    println!("─────────────────────────────────────");
    println!("{}", test_text);

    println!("\n📤 脱敏后 ({})", sanitized.len());
    println!("─────────────────────────────────────");
    println!("{}", sanitized);

    println!("\n📊 统计:");
    let total = map.entries.len();
    println!("   共 {} 处敏感数据：", total);
    for (entity_type, count) in &stats {
        let label = match entity_type.as_str() {
            "PERSON" => "人名",
            "ORG" => "公司/组织",
            "PHONE" => "手机号",
            "ID_CARD" => "身份证号",
            "BANK" => "银行卡号",
            "EMAIL" => "邮箱",
            "ADDR" => "地址",
            "URL" => "URL",
            "IP" => "IP地址",
            "MONEY" => "金额",
            _ => entity_type,
        };
        println!("   {:<10} : {:>2} 处  {}", entity_type, count, label);
    }

    println!("\n📌 映射表:");
    for entry in &map.entries {
        println!("   {:<15} ←  {}  ({})", entry.placeholder, entry.original, entry.entity_label);
    }

    // 还原验证
    let restored = map.restore(&sanitized);
    let ok = restored == test_text;
    println!("\n✅ 还原验证: {}", if ok { "完全匹配 ✓" } else { "有差异 ✗" });

    if !ok {
        // 找不同
        let min_len = restored.len().min(test_text.len());
        for i in 0..min_len {
            if restored.as_bytes()[i] != test_text.as_bytes()[i] {
                let start = if i > 20 { i - 20 } else { 0 };
                let end = (i + 30).min(min_len);
                println!("\n   差异位置 {}:", i);
                println!("   原文: ...{}...", &test_text[start..end]);
                println!("   还原: ...{}...", &restored[start..end]);
                break;
            }
        }
        if restored.len() != test_text.len() {
            println!("   长度不同: 还原 {} vs 原文 {}", restored.len(), test_text.len());
        }
    }
}

/// 根据文件扩展名自动选择解析方式
fn read_file_content(path: &PathBuf) -> Result<String, String> {
    let lower = path.to_string_lossy().to_lowercase();

    if lower.ends_with(".xlsx") {
        // Excel 用 calamine 读取后转为文本
        let path_str = path.to_str().unwrap();
        let data = excel_parser::read_file(path_str)
            .map_err(|e| format!("读取 Excel 失败: {}", e))?;
        let mut buf = String::new();
        // 表头
        buf.push_str(&data.columns.iter().map(|c| c.name.as_str()).collect::<Vec<_>>().join("\t"));
        buf.push('\n');
        // 数据行
        for row in &data.rows {
            buf.push_str(&row.join("\t"));
            buf.push('\n');
        }
        Ok(buf)
    } else if lower.ends_with(".xls") || lower.ends_with(".et") {
        // .xls 和 WPS .et 格式 — calamine 尝试解析
        let path_str = path.to_str().unwrap();
        match excel_parser::read_file(path_str) {
            Ok(data) => {
                let mut buf = String::new();
                buf.push_str(&data.columns.iter().map(|c| c.name.as_str()).collect::<Vec<_>>().join("\t"));
                buf.push('\n');
                for row in &data.rows {
                    buf.push_str(&row.join("\t"));
                    buf.push('\n');
                }
                Ok(buf)
            }
            Err(e) => {
                // calamine 无法解析时，尝试作为二进制文件读取
                eprintln!("⚠️  calamine 解析失败: {}，尝试直接读取...", e);
                fs::read_to_string(path)
                    .map_err(|_| format!("无法解析文件: {}", path.to_string_lossy()))
            }
        }
    } else if lower.ends_with(".csv") {
        let path_str = path.to_str().unwrap();
        let data = excel_parser::read_csv(path_str)
            .map_err(|e| format!("读取 CSV 失败: {}", e))?;
        let mut buf = String::new();
        buf.push_str(&data.columns.iter().map(|c| c.name.as_str()).collect::<Vec<_>>().join("\t"));
        buf.push('\n');
        for row in &data.rows {
            buf.push_str(&row.join("\t"));
            buf.push('\n');
        }
        Ok(buf)
    } else if lower.ends_with(".pdf") {
        pdf_parser::extract_text(path.to_str().unwrap())
    } else if lower.ends_with(".docx") {
        docx_parser::extract_text(path.to_str().unwrap())
    } else if lower.ends_with(".doc") {
        // .doc 旧版 Word（二进制格式），调用 catdoc 或 libreoffice
        convert_doc_to_text(path.to_str().unwrap())
    } else if lower.ends_with(".wps") {
        // .wps WPS文字格式，调用 libreoffice 转换
        convert_wps_to_text(path.to_str().unwrap())
    } else if is_image_file(&lower) {
        // 图片 OCR
        ocr_image(path.to_str().unwrap())
    } else {
        // TXT/其他文本文件
        fs::read_to_string(path)
            .map_err(|e| format!("读取文件失败: {}", e))
    }
}

/// Excel 解析模块（CLI 专用，复用 audit-desktop 的逻辑）
mod excel_parser {
    use calamine::{open_workbook, Reader, Xlsx, Xls};
    use serde::Serialize;

    #[derive(Clone, Serialize)]
    pub struct ColumnInfo {
        pub name: String,
        pub sample_values: Vec<String>,
    }

    #[derive(Clone, Serialize)]
    pub struct ExcelData {
        pub sheet_name: String,
        pub columns: Vec<ColumnInfo>,
        pub rows: Vec<Vec<String>>,
        pub total_rows: usize,
    }

    pub fn read_file(path: &str) -> Result<ExcelData, String> {
        let lower = path.to_lowercase();
        if lower.ends_with(".xlsx") {
            read_xlsx(path)
        } else if lower.ends_with(".xls") {
            read_xls(path)
        } else if lower.ends_with(".et") {
            read_xls(path).or_else(|_| read_xlsx(path))
        } else if lower.ends_with(".csv") {
            read_csv(path)
        } else {
            Err(format!("不支持的文件格式: {}", path))
        }
    }

    fn read_xls(path: &str) -> Result<ExcelData, String> {
        let mut workbook: Xls<_> = open_workbook(path)
            .map_err(|e| format!("无法打开 .xls 文件: {}", e))?;
        let sheet_names = workbook.sheet_names().to_vec();
        if sheet_names.is_empty() {
            return Err(format!("文件 '{}' 中没有工作表", path));
        }
        let sheet_name = sheet_names[0].clone();
        let range = workbook.worksheet_range(&sheet_name)
            .map_err(|e| format!("读取工作表失败: {}", e))?;
        let mut rows_iter = range.rows();
        let headers: Vec<String> = match rows_iter.next() {
            Some(row) => row.iter().map(|c| c.to_string()).collect(),
            None => return Err("工作表为空".into()),
        };
        let col_count = headers.len();
        let mut columns: Vec<ColumnInfo> = headers.iter()
            .map(|h| ColumnInfo { name: h.clone(), sample_values: Vec::new() })
            .collect();
        let mut rows: Vec<Vec<String>> = Vec::new();
        for row in rows_iter {
            let values: Vec<String> = (0..col_count)
                .map(|i| row.get(i).map(|c| c.to_string()).unwrap_or_default())
                .collect();
            for (i, v) in values.iter().enumerate() {
                if !v.is_empty() && columns[i].sample_values.len() < 5 {
                    columns[i].sample_values.push(v.clone());
                }
            }
            rows.push(values);
        }
        let total_rows = rows.len();
        Ok(ExcelData { sheet_name, columns, rows, total_rows })
    }

    fn read_xlsx(path: &str) -> Result<ExcelData, String> {
        let mut workbook: Xlsx<_> = open_workbook(path)
            .map_err(|e| format!("无法打开 .xlsx 文件: {}", e))?;
        let sheet_names = workbook.sheet_names().to_vec();
        if sheet_names.is_empty() {
            return Err(format!("文件 '{}' 中没有工作表", path));
        }
        let sheet_name = sheet_names[0].clone();
        let range = workbook.worksheet_range(&sheet_name)
            .map_err(|e| format!("读取工作表失败: {}", e))?;
        let mut rows_iter = range.rows();
        let headers: Vec<String> = match rows_iter.next() {
            Some(row) => row.iter().map(|c| c.to_string()).collect(),
            None => return Err("工作表为空".into()),
        };
        let col_count = headers.len();
        let mut columns: Vec<ColumnInfo> = headers.iter()
            .map(|h| ColumnInfo { name: h.clone(), sample_values: Vec::new() })
            .collect();
        let mut rows: Vec<Vec<String>> = Vec::new();
        for row in rows_iter {
            let values: Vec<String> = (0..col_count)
                .map(|i| row.get(i).map(|c| c.to_string()).unwrap_or_default())
                .collect();
            for (i, v) in values.iter().enumerate() {
                if !v.is_empty() && columns[i].sample_values.len() < 5 {
                    columns[i].sample_values.push(v.clone());
                }
            }
            rows.push(values);
        }
        let total_rows = rows.len();
        Ok(ExcelData { sheet_name, columns, rows, total_rows })
    }

    pub fn read_csv(path: &str) -> Result<ExcelData, String> {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_path(path)
            .map_err(|e| format!("无法读取 CSV: {}", e))?;
        let headers: Vec<String> = reader.headers()
            .map_err(|e| format!("读取表头失败: {}", e))?
            .iter().map(|h| h.to_string()).collect();
        let col_count = headers.len();
        let mut columns: Vec<ColumnInfo> = headers.iter()
            .map(|h| ColumnInfo { name: h.clone(), sample_values: Vec::new() })
            .collect();
        let mut rows: Vec<Vec<String>> = Vec::new();
        for result in reader.records() {
            let record = result.map_err(|e| format!("读取行失败: {}", e))?;
            let values: Vec<String> = (0..col_count)
                .map(|i| record.get(i).map(|s| s.to_string()).unwrap_or_default())
                .collect();
            for (i, v) in values.iter().enumerate() {
                if !v.is_empty() && columns[i].sample_values.len() < 5 {
                    columns[i].sample_values.push(v.clone());
                }
            }
            rows.push(values);
        }
        let total_rows = rows.len();
        Ok(ExcelData { sheet_name: "Sheet1".into(), columns, rows, total_rows })
    }
}

/// 从 YAML 文件加载自定义敏感词表
fn load_custom_dict(path: &PathBuf) -> Result<HashMap<String, Vec<String>>, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("读取文件失败: {}", e))?;

    let dict: CustomDict = serde_yaml::from_str(&content)
        .map_err(|e| format!("解析 YAML 失败: {}", e))?;

    let mut result: HashMap<String, Vec<String>> = HashMap::new();

    if let Some(persons) = dict.persons {
        if !persons.is_empty() {
            result.insert("PERSON".to_string(), persons);
        }
    }
    if let Some(orgs) = dict.orgs {
        if !orgs.is_empty() {
            result.insert("ORG".to_string(), orgs);
        }
    }
    if let Some(projects) = dict.projects {
        if !projects.is_empty() {
            result.insert("PROJECT".to_string(), projects);
        }
    }
    if let Some(systems) = dict.systems {
        if !systems.is_empty() {
            result.insert("SYSTEM".to_string(), systems);
        }
    }
    if let Some(partners) = dict.partners {
        if !partners.is_empty() {
            result.insert("PARTNER".to_string(), partners);
        }
    }

    Ok(result)
}
