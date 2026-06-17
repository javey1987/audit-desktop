/// Excel 读取模块 — 使用 calamine 原生读 .xlsx，不经过 CSV 科学计数法坑

use calamine::{open_workbook, Reader, Xlsx};
use serde::Serialize;

/// 一列数据的信息
#[derive(Debug, Clone, Serialize)]
pub struct ColumnInfo {
    pub name: String,
    pub sample_values: Vec<String>, // 前5个样本值
}

/// 读取结果
#[derive(Debug, Clone, Serialize)]
pub struct ExcelData {
    pub sheet_name: String,
    pub columns: Vec<ColumnInfo>,
    pub rows: Vec<Vec<String>>,  // 不含表头
    pub total_rows: usize,
}

/// 打开 .xlsx 文件并读取第一个工作表
pub fn read_xlsx(path: &str) -> Result<ExcelData, String> {
    let mut workbook: Xlsx<_> = open_workbook(path)
        .map_err(|e| format!("无法打开文件: {}", e))?;

    let sheet_names = workbook.sheet_names().to_vec();
    if sheet_names.is_empty() {
        return Err("工作簿中没有工作表".into());
    }

    let sheet_name = sheet_names[0].clone();
    let range = workbook.worksheet_range(&sheet_name)
        .map_err(|e| format!("读取工作表失败: {}", e))?;

    let mut rows_iter = range.rows();

    // 第一行为表头
    let headers: Vec<String> = match rows_iter.next() {
        Some(row) => row.iter().map(|c| c.to_string()).collect(),
        None => return Err("工作表为空".into()),
    };

    let col_count = headers.len();
    let mut columns: Vec<ColumnInfo> = headers.iter()
        .map(|h| ColumnInfo {
            name: h.clone(),
            sample_values: Vec::new(),
        })
        .collect();

    let mut rows: Vec<Vec<String>> = Vec::new();
    for row in rows_iter {
        let values: Vec<String> = (0..col_count)
            .map(|i| row.get(i).map(|c| c.to_string()).unwrap_or_default())
            .collect();

        // 收集样本值（每列前5个非空值）
        for (i, v) in values.iter().enumerate() {
            if !v.is_empty() && columns[i].sample_values.len() < 5 {
                columns[i].sample_values.push(v.clone());
            }
        }

        rows.push(values);
    }

    let total_rows = rows.len();
    Ok(ExcelData {
        sheet_name,
        columns,
        rows,
        total_rows,
    })
}

/// 读取 CSV 文件
pub fn read_csv(path: &str) -> Result<ExcelData, String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)
        .map_err(|e| format!("无法读取 CSV: {}", e))?;

    let headers: Vec<String> = reader.headers()
        .map_err(|e| format!("读取表头失败: {}", e))?
        .iter()
        .map(|h| h.to_string())
        .collect();

    let col_count = headers.len();
    let mut columns: Vec<ColumnInfo> = headers.iter()
        .map(|h| ColumnInfo {
            name: h.clone(),
            sample_values: Vec::new(),
        })
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
    Ok(ExcelData {
        sheet_name: "Sheet1".into(),
        columns,
        rows,
        total_rows,
    })
}

/// 自动检测文件格式并读取
pub fn read_file(path: &str) -> Result<ExcelData, String> {
    let lower = path.to_lowercase();
    if lower.ends_with(".csv") {
        read_csv(path)
    } else if lower.ends_with(".xlsx") {
        read_xlsx(path)
    } else {
        Err(format!("不支持的文件格式: {}", path))
    }
}
