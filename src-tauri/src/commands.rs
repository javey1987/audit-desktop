/// Tauri 命令 — 前端调用的后端接口

use serde::Serialize;
use std::sync::Mutex;
use tauri::State;

use crate::desensitize::{self, MatchedColumn};
use crate::excel::{self, ExcelData};
use crate::llm;

/// 应用状态
pub struct AppState {
    pub current_data: Mutex<Option<ExcelData>>,
    pub api_key: Mutex<String>,
}

#[derive(Debug, Serialize)]
pub struct FileInfo {
    pub sheet_name: String,
    pub total_rows: usize,
    pub columns: Vec<ColumnSummary>,
}

#[derive(Debug, Serialize)]
pub struct ColumnSummary {
    pub name: String,
    pub sensitive_type: String,
    pub sensitive_label: String,
    pub sample_values: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct DesensitizeResult {
    pub columns: Vec<ColumnSummary>,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub matched_count: usize,
}

#[derive(Debug, Serialize)]
pub struct ChatResult {
    pub reply: String,
}

// ============================================================
//  命令实现
// ============================================================

/// 打开 Excel/CSV 文件，解析并返回列信息
#[tauri::command]
pub fn open_excel(path: String, state: State<AppState>) -> Result<FileInfo, String> {
    let data = excel::read_file(&path)?;

    let matched = desensitize::match_columns(&data.columns);

    let columns: Vec<ColumnSummary> = data
        .columns
        .iter()
        .map(|c| {
            let mt = matched.iter().find(|m| m.column_name == c.name);
            ColumnSummary {
                name: c.name.clone(),
                sensitive_type: mt
                    .map(|m| m.sensitive_type.prefix())
                    .unwrap_or("")
                    .to_string(),
                sensitive_label: mt
                    .map(|m| m.sensitive_type.label())
                    .unwrap_or("未识别")
                    .to_string(),
                sample_values: c.sample_values.clone(),
            }
        })
        .collect();

    *state.current_data.lock().unwrap() = Some(data);

    Ok(FileInfo {
        sheet_name: String::new(),
        total_rows: 0,
        columns,
    })
}

/// 扫描敏感数据（按列匹配 + 内容正则）
#[tauri::command]
pub fn scan_sensitive(path: String) -> Result<Vec<MatchedColumn>, String> {
    let data = excel::read_file(&path)?;
    let matched = desensitize::match_columns(&data.columns);
    Ok(matched)
}

/// 对文件执行脱敏，返回脱敏后的表格数据
#[tauri::command]
pub fn desensitize_file(path: String) -> Result<DesensitizeResult, String> {
    let data = excel::read_file(&path)?;
    let headers: Vec<String> = data.columns.iter().map(|c| c.name.clone()).collect();
    let (sanitized_rows, matched) = desensitize::desensitize_data(&data.columns, &data.rows);

    let matched_count = matched.iter().filter(|m| m.sensitive_type.prefix() != "").count();

    let columns: Vec<ColumnSummary> = matched
        .into_iter()
        .map(|m| ColumnSummary {
            name: m.column_name,
            sensitive_type: m.sensitive_type.prefix().to_string(),
            sensitive_label: m.sensitive_type.label().to_string(),
            sample_values: m.sample_values,
        })
        .collect();

    Ok(DesensitizeResult {
        columns,
        headers,
        rows: sanitized_rows,
        matched_count,
    })
}

/// 对话审计（脱敏后的数据 + DeepSeek）
#[tauri::command]
pub async fn chat_with_llm(
    message: String,
    file_path: String,
    state: State<'_, AppState>,
) -> Result<ChatResult, String> {
    let api_key = state.api_key.lock().unwrap().clone();
    if api_key.is_empty() {
        return Err("请先设置 DeepSeek API Key".into());
    }

    // 读取文件数据
    let data = excel::read_file(&file_path)?;
    let headers: Vec<String> = data.columns.iter().map(|c| c.name.clone()).collect();
    let (sanitized_rows, matched) = desensitize::desensitize_data(&data.columns, &data.rows);

    // 构建脱敏后的数据摘要
    let file_info = format!(
        "文件：{}\n列数：{}\n行数：{}",
        file_path,
        data.columns.len(),
        data.total_rows
    );

    let matched_desc: String = matched
        .iter()
        .filter(|m| m.sensitive_type.prefix() != "")
        .map(|m| format!("  - {}: {} ({})", m.column_name, m.sensitive_type.label(), m.sensitive_type.prefix()))
        .collect::<Vec<_>>()
        .join("\n");

    let system_prompt = llm::build_system_prompt(&file_info, &matched_desc);

    // 取前10行脱敏数据作为上下文
    let data_sample: String = sanitized_rows
        .iter()
        .take(10)
        .map(|row| {
            headers
                .iter()
                .zip(row.iter())
                .map(|(h, v)| format!("{}={}", h, v))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let user_message = format!(
        "{}\n\n数据样本（脱敏后前10行）：\n{}",
        message, data_sample
    );

    let reply = llm::chat(
        vec![
            ("system".into(), system_prompt),
            ("user".into(), user_message),
        ],
        &api_key,
    )
    .await?;

    Ok(ChatResult {
        reply: reply.content,
    })
}
