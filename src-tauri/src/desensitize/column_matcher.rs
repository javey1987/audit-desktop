/// 列名匹配器 — 根据列名自动识别敏感字段并生成脱敏规则
///
/// 借鉴了审计砖家后端 data_sanitizer.py 的思路，但重构为 Rust 版本

use serde::Serialize;
use std::collections::HashMap;

use crate::excel::ColumnInfo;

/// 敏感列类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub enum ColumnSensitiveType {
    Person,       // 人名
    Company,      // 公司/单位
    Phone,        // 手机/电话
    IdCard,       // 身份证
    BankCard,     // 银行卡号/账号
    Email,        // 邮箱
    Address,      // 地址
    Region,       // 地区
    GovDept,      // 党政机关部门
    Money,        // 金额
    Date,         // 日期
    Project,      // 项目
    Other,        // 不敏感
}

impl ColumnSensitiveType {
    pub fn prefix(&self) -> &str {
        match self {
            ColumnSensitiveType::Person => "PERSON",
            ColumnSensitiveType::Company => "COMPANY",
            ColumnSensitiveType::Phone => "PHONE",
            ColumnSensitiveType::IdCard => "ID_CARD",
            ColumnSensitiveType::BankCard => "BANK_CARD",
            ColumnSensitiveType::Email => "EMAIL",
            ColumnSensitiveType::Address => "ADDR",
            ColumnSensitiveType::Region => "REGION",
            ColumnSensitiveType::GovDept => "GOVDEPT",
            ColumnSensitiveType::Money => "MONEY",
            ColumnSensitiveType::Date => "DATE",
            ColumnSensitiveType::Project => "PROJECT",
            ColumnSensitiveType::Other => "",
        }
    }
    pub fn label(&self) -> &str {
        match self {
            ColumnSensitiveType::Person => "人名",
            ColumnSensitiveType::Company => "公司/单位",
            ColumnSensitiveType::Phone => "手机/电话",
            ColumnSensitiveType::IdCard => "身份证号",
            ColumnSensitiveType::BankCard => "银行卡号/账号",
            ColumnSensitiveType::Email => "邮箱",
            ColumnSensitiveType::Address => "地址",
            ColumnSensitiveType::Region => "地区",
            ColumnSensitiveType::GovDept => "党政机关部门",
            ColumnSensitiveType::Money => "金额",
            ColumnSensitiveType::Date => "日期",
            ColumnSensitiveType::Project => "项目",
            ColumnSensitiveType::Other => "",
        }
    }
}

/// 敏感列匹配结果
#[derive(Debug, Clone, Serialize)]
pub struct MatchedColumn {
    pub column_name: String,
    pub sensitive_type: ColumnSensitiveType,
    pub sample_values: Vec<String>,
}

/// 列名匹配规则表
struct ColumnRule {
    keywords: Vec<&'static str>,
    sens_type: ColumnSensitiveType,
}

fn build_column_rules() -> Vec<ColumnRule> {
    vec![
        ColumnRule {
            keywords: vec!["姓名", "名字", "名称", "录入人", "收款人", "付款人", "联系人",
                          "负责人", "客户名", "name", "customer", "contact", "person"],
            sens_type: ColumnSensitiveType::Person,
        },
        ColumnRule {
            keywords: vec!["单位", "公司", "企业", "部门", "供应商", "机构",
                          "company", "org", "vendor", "dept", "department", "organization"],
            sens_type: ColumnSensitiveType::Company,
        },
        ColumnRule {
            keywords: vec!["手机", "电话", "手机号", "电话号码", "联系电话",
                          "phone", "tel", "mobile"],
            sens_type: ColumnSensitiveType::Phone,
        },
        ColumnRule {
            keywords: vec!["身份证", "证件号", "idcard", "id_card", "id_number"],
            sens_type: ColumnSensitiveType::IdCard,
        },
        ColumnRule {
            keywords: vec!["银行卡", "账号", "卡号", "开户行", "银行账号", "付款人账号", "收款人账号",
                          "bank", "account", "card"],
            sens_type: ColumnSensitiveType::BankCard,
        },
        ColumnRule {
            keywords: vec!["邮箱", "邮件", "email", "mail"],
            sens_type: ColumnSensitiveType::Email,
        },
        ColumnRule {
            keywords: vec!["地址", "住址", "address", "location"],
            sens_type: ColumnSensitiveType::Address,
        },
        ColumnRule {
            keywords: vec!["地区", "区域", "省份", "省市", "区划", "地域", "region", "district"],
            sens_type: ColumnSensitiveType::Region,
        },
        ColumnRule {
            keywords: vec!["部门", "处室", "科室", "机关", "单位名称", "机构",
                          "dept", "department", "bureau", "office"],
            sens_type: ColumnSensitiveType::GovDept,
        },
        ColumnRule {
            keywords: vec!["金额", "总额", "总金额", "价格", "单价", "费用", "支出", "收入",
                          "money", "amount", "price", "cost", "fee", "total", "sum",
                          "budget", "payment"],
            sens_type: ColumnSensitiveType::Money,
        },
        ColumnRule {
            keywords: vec!["日期", "时间", "年月", "date", "time", "year", "month",
                          "回单时间", "清算时间", "录入时间"],
            sens_type: ColumnSensitiveType::Date,
        },
        ColumnRule {
            keywords: vec!["项目", "工程", "计划", "方案", "指标",
                          "project", "mission"],
            sens_type: ColumnSensitiveType::Project,
        },
    ]
}

/// 匹配列名
pub fn match_columns(columns: &[ColumnInfo]) -> Vec<MatchedColumn> {
    let rules = build_column_rules();
    let mut results = Vec::new();

    for col in columns {
        let col_lower = col.name.to_lowercase();
        let mut matched = ColumnSensitiveType::Other;

        for rule in &rules {
            if rule.keywords.iter().any(|kw| col_lower.contains(kw)) {
                matched = rule.sens_type.clone();
                break;
            }
        }

        results.push(MatchedColumn {
            column_name: col.name.clone(),
            sensitive_type: matched,
            sample_values: col.sample_values.clone(),
        });
    }

    results
}

/// 对列值做替换式脱敏（可逆）
pub fn desensitize_value(value: &str, sens_type: &ColumnSensitiveType) -> String {
    if value.is_empty() || value == "-" {
        return value.to_string();
    }

    match sens_type {
        // 人名：保留姓
        ColumnSensitiveType::Person => {
            let v = value.trim();
            let chars: Vec<char> = v.chars().collect();
            if chars.len() >= 2 {
                format!("{}某", chars[0])
            } else if chars.len() == 1 {
                "某".into()
            } else {
                value.to_string()
            }
        }
        // 公司名：保留前2字
        ColumnSensitiveType::Company => {
            let v = value.trim();
            let chars: Vec<char> = v.chars().collect();
            if chars.len() >= 4 {
                format!("{}**", &v[..chars[2].len_utf8() * 2])
            } else if chars.len() >= 2 {
                format!("{}**", &v[..chars[0].len_utf8()])
            } else {
                "**".into()
            }
        }
        // 手机号：保留前3后2
        ColumnSensitiveType::Phone => {
            let digits: String = value.chars().filter(|c| c.is_ascii_digit()).collect();
            if digits.len() >= 7 {
                format!("{}****{}", &digits[..3], &digits[digits.len()-2..])
            } else if digits.len() >= 4 {
                format!("{}****", &digits[..3])
            } else {
                "****".into()
            }
        }
        // 身份证：保留前6后4
        ColumnSensitiveType::IdCard => {
            let v = value.trim();
            if v.len() >= 10 {
                format!("{}********{}", &v[..6], &v[v.len()-4..])
            } else {
                "********".into()
            }
        }
        // 银行卡号/账号：保留前4后4
        ColumnSensitiveType::BankCard => {
            let v = value.trim();
            if v.len() >= 8 {
                format!("{}****{}", &v[..4], &v[v.len()-4..])
            } else {
                "********".into()
            }
        }
        // 邮箱：替换用户名
        ColumnSensitiveType::Email => {
            if let Some(at_pos) = value.find('@') {
                let local = &value[..at_pos];
                let domain = &value[at_pos..];
                if local.len() >= 3 {
                    format!("{}***{}", &local[..1], domain)
                } else {
                    format!("***{}", domain)
                }
            } else {
                "***".into()
            }
        }
        // 地址：保留前6字
        ColumnSensitiveType::Address => {
            let v = value.trim();
            let chars: Vec<char> = v.chars().collect();
            if chars.len() >= 6 {
                let cut: String = chars[..6].iter().collect();
                format!("{}****", cut)
            } else {
                "****".into()
            }
        }
        // 金额：审计核心比对数据，不脱敏
        ColumnSensitiveType::Money => value.to_string(),
        // 地区名：替换为"某地区"
        ColumnSensitiveType::Region => {
            let v = value.trim();
            let chars: Vec<char> = v.chars().collect();
            if chars.len() >= 3 {
                let prefix: String = chars[..3].iter().collect();
                format!("{}某某", prefix)
            } else {
                "某地区".into()
            }
        }
        // 党政机关部门：替换为"某部门"
        ColumnSensitiveType::GovDept => "某部门".into(),
        // 日期/时间/项目：不脱敏（通常不敏感）
        ColumnSensitiveType::Date | ColumnSensitiveType::Project => value.to_string(),
        ColumnSensitiveType::Other => value.to_string(),
    }
}

/// 对整个数据文件执行列匹配脱敏
/// 返回：脱敏后的行 + 匹配信息
pub fn desensitize_data(
    columns: &[ColumnInfo],
    rows: &[Vec<String>],
) -> (Vec<Vec<String>>, Vec<MatchedColumn>) {
    let matched = match_columns(columns);
    let mut result = Vec::new();

    for row in rows {
        let mut new_row = Vec::new();
        for (i, val) in row.iter().enumerate() {
            if i < matched.len() {
                new_row.push(desensitize_value(val, &matched[i].sensitive_type));
            } else {
                new_row.push(val.clone());
            }
        }
        result.push(new_row);
    }

    (result, matched)
}
