/// 敏感数据类型与正则规则
/// 每种类型对应一种实体：PERSON, PHONE, ID_CARD, BANK_CARD, EMAIL, IP, URL, MONEY, PROJECT

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 脱敏级别
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SensitiveLevel {
    /// 替换为标记（默认）
    Mask,
    /// 保留原值，仅标记类型
    Keep,
}

/// 实体类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityType {
    Person,
    Org,
    Phone,
    IdCard,
    BankCard,
    Email,
    Address,
    Region,
    GovDept,
    Url,
    Ip,
    Money,
    Project,
    Custom(String),
}

impl EntityType {
    pub fn prefix(&self) -> &str {
        match self {
            EntityType::Person => "PERSON",
            EntityType::Org => "ORG",
            EntityType::Phone => "PHONE",
            EntityType::IdCard => "ID_CARD",
            EntityType::BankCard => "BANK",
            EntityType::Email => "EMAIL",
            EntityType::Address => "ADDR",
            EntityType::Region => "REGION",
            EntityType::GovDept => "GOVDEPT",
            EntityType::Url => "URL",
            EntityType::Ip => "IP",
            EntityType::Money => "MONEY",
            EntityType::Project => "PROJECT",
            EntityType::Custom(s) => s.as_str(),
        }
    }

    pub fn label(&self) -> &str {
        match self {
            EntityType::Person => "人名",
            EntityType::Org => "公司/组织",
            EntityType::Phone => "手机号",
            EntityType::IdCard => "身份证号",
            EntityType::BankCard => "银行卡号",
            EntityType::Email => "邮箱",
            EntityType::Address => "地址",
            EntityType::Region => "地区",
            EntityType::GovDept => "党政机关部门",
            EntityType::Url => "URL",
            EntityType::Ip => "IP地址",
            EntityType::Money => "金额",
            EntityType::Project => "项目名",
            EntityType::Custom(s) => s,
        }
    }

    /// 获取脱敏级别
    pub fn level(&self) -> SensitiveLevel {
        match self {
            EntityType::Money | EntityType::Project | EntityType::Ip => SensitiveLevel::Keep,
            _ => SensitiveLevel::Mask,
        }
    }
}

/// 敏感数据匹配结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    pub original: String,
    pub start: usize,
    pub end: usize,
    pub entity_type: EntityType,
}

/// 扫描器 — 内置正则规则
pub struct Scanner {
    patterns: Vec<(EntityType, Regex)>,
    /// 自定义词典：(entity_type_label, terms)
    custom_dict: HashMap<String, Vec<String>>,
}

impl Scanner {
    pub fn new() -> Self {
        let patterns = build_builtin_patterns();
        Scanner {
            patterns,
            custom_dict: HashMap::new(),
        }
    }

    /// 加载自定义敏感词表
    pub fn load_custom_dict(&mut self, dict: HashMap<String, Vec<String>>) {
        self.custom_dict = dict;
    }

    /// 扫描一段文本，返回所有发现的敏感信息
    pub fn scan(&self, text: &str) -> Vec<MatchResult> {
        let mut results = Vec::new();

        // 1. 正则扫描
        for (entity_type, re) in &self.patterns {
            for m in re.find_iter(text) {
                results.push(MatchResult {
                    original: m.as_str().to_string(),
                    start: m.start(),
                    end: m.end(),
                    entity_type: entity_type.clone(),
                });
            }
        }

        // 2. 自定义词典扫描（简单字符串匹配，不行正则）
        let text_bytes = text.as_bytes();
        let text_len = text.len();
        for (type_label, terms) in &self.custom_dict {
            for term in terms {
                let term_bytes = term.as_bytes();
                let term_len = term.len();
                if term_len == 0 || term_len > text_len {
                    continue;
                }

                // 在文本中搜索词条
                let mut pos = 0;
                while let Some(found) = text[pos..].find(term) {
                    let abs_pos = pos + found;
                    let abs_end = abs_pos + term_len;

                    // 检查边界：确保不是某个更长中文词的一部分
                    let has_left_boundary = abs_pos == 0
                        || !is_chinese_char(text_bytes[abs_pos - 1]);
                    let has_right_boundary = abs_end >= text_len
                        || !is_chinese_char(text_bytes[abs_end]);

                    if has_left_boundary && has_right_boundary {
                        results.push(MatchResult {
                            original: term.to_string(),
                            start: abs_pos,
                            end: abs_end,
                            entity_type: EntityType::Custom(type_label.clone()),
                        });
                    }

                    pos = abs_pos + term_len;
                    if pos >= text_len {
                        break;
                    }
                }
            }
        }

        // 3. 按位置排序
        results.sort_by_key(|r| r.start);

        // 4. 去重（重叠的只保留第一个长的）
        dedup_overlapping(&mut results);

        results
    }
}

/// 判断一个字节是否是中文字符的一部分（UTF-8 多字节序列，首字节 ≥ 0xE4 通常是中文）
fn is_chinese_char(b: u8) -> bool {
    // Unicode CJK Unified Ideographs 起始字节范围
    b >= 0xE4 && b <= 0xE9
}

/// 去重重叠匹配
fn dedup_overlapping(results: &mut Vec<MatchResult>) {
    if results.is_empty() {
        return;
    }
    let mut i = 0;
    while i < results.len() {
        let mut j = i + 1;
        while j < results.len() {
            if results[j].start < results[i].end {
                // 重叠时保留更长的
                let i_len = results[i].original.len();
                let j_len = results[j].original.len();
                if j_len > i_len {
                    results.swap(i, j);
                }
                results.remove(j);
            } else {
                j += 1;
            }
        }
        i += 1;
    }
}

/// 构建内置正则规则
fn build_builtin_patterns() -> Vec<(EntityType, Regex)> {
    let mut compiled: Vec<(EntityType, Regex)> = Vec::new();

    // 辅助：添加规则
    let mut add = |et: EntityType, pattern: &str|  {
                match regex::RegexBuilder::new(pattern).unicode(true).build() {
            Ok(re) => compiled.push((et, re)),
            Err(e) => eprintln!("[warn] 正则编译失败: {} — {}", pattern, e),
            Err(e) => eprintln!("[warn] 正则编译失败: {} — {}", pattern, e),
        }
    };

    // ---- 手机号 ----
    add(EntityType::Phone, r"1[3-9]\d{9}");
    add(EntityType::Phone, r"0\d{2,3}[-\s]?\d{7,8}");
    add(EntityType::Phone, r"(\+86)[-\s]?1[3-9]\d{9}");

    // ---- 身份证 ----
    add(EntityType::IdCard, r"[1-9]\d{5}(?:19|20)\d{2}(?:0[1-9]|1[0-2])(?:0[1-9]|[12]\d|3[01])\d{3}[\dXx]");
    add(EntityType::IdCard, r"[1-9]\d{7}(?:0[1-9]|1[0-2])(?:0[1-9]|[12]\d|3[01])\d{3}");

    // ---- 银行卡 ----
    add(EntityType::BankCard, r"[34569]\d{14,18}");

    // ---- 邮箱 ----
    add(EntityType::Email, r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}");

    // ---- IP地址 ----
    add(EntityType::Ip, r"\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}");

    // ---- URL ----
    add(EntityType::Url, r"https?://[^\s,，。；;）\)\]】}]+");

    // ---- 金额 ----
    add(EntityType::Money, r"[¥￥]\s*[\d,]+(?:\.\d+)?(?:\s*[万亿])?");
    add(EntityType::Money, r"\d+(?:,\d{3})*(?:\.\d+)?\s*(?:元|美元|欧元|英镑|日元|万|亿)");
    add(EntityType::Money, r"[零壹贰叁肆伍陆柒捌玖拾佰仟万亿]+[元整]");

    // ---- 中文人名（上下文相关的简单匹配） ----
    // "姓名：xxx"
    add(EntityType::Person, r"(?:姓名|名字|联系人|负责人|客户名)[：:]\s*[\u4e00-\u9fff]{2,4}");
    // "叫xxx"
    add(EntityType::Person, r"叫[\u4e00-\u9fff]{2,4}");
    // "xxx的电话"
    add(EntityType::Person, r"[\u4e00-\u9fff]{2,4}的电话");

    // ---- 地址 ----
    add(EntityType::Address,
        r"(?:北京|天津|上海|重庆|河北|山西|辽宁|吉林|黑龙江|江苏|浙江|安徽|福建|江西|山东|河南|湖北|湖南|广东|海南|四川|贵州|云南|陕西|甘肃|青海|台湾|内蒙古|广西|西藏|宁夏|新疆|香港|澳门)[\u4e00-\u9fff]{2,}(?:省|市|区|县|镇|乡|街道|路|街|巷|大道)[\u4e00-\u9fff\d\-]{0,}(?:号|号院|号楼|室|单元)"
    );

    // ---- 地区名（审计场景中可推断被审计单位位置） ----
    add(EntityType::Region,
        r"(?:北京|天津|上海|重庆|河北|山西|辽宁|吉林|黑龙江|江苏|浙江|安徽|福建|江西|山东|河南|湖北|湖南|广东|海南|四川|贵州|云南|陕西|甘肃|青海|台湾|内蒙古|广西|西藏|宁夏|新疆|香港|澳门)(?:省|市|自治区|特别行政区)");
    add(EntityType::Region,
        r"[\u4e00-\u9fff]{2,}(?:市|区|县|镇|乡|街道)(?:办事处)?");

    // ---- 党政机关部门 ----
    add(EntityType::GovDept,
        r"[\u4e00-\u9fff]{2,}(?:局|委|办|部|厅|处|中心|所|站|院|队|会|社|集团)");
    add(EntityType::GovDept,
        r"(?:中共|国务院|中央|国家|省|市|县|区)[\u4e00-\u9fff]{1,}(?:委员会|办公室|管理局|指挥部|领导小组|工作组)");

    // ---- 公司/组织名 ----
    add(EntityType::Org, r"(?:供应商|客户|甲方|乙方|承包方|采购方|委托方|投标方)[：:]\s*[\u4e00-\u9fff]{2,20}(?:公司|企业|集团|厂)?");
    add(EntityType::Org, r"[\u4e00-\u9fff]{2,10}(?:有限公司|有限责任公司|股份有限公司|集团有限公司)");
    // 纯公司名匹配需要至少4个字才能算公司
    add(EntityType::Org, r"[\u4e00-\u9fff]{4,8}(?:公司|集团|企业)");
    add(EntityType::Org, r"(?:单位|公司名称|企业名称|组织名称)[：:]\s*[\u4e00-\u9fff]{2,20}");
    // ---- 项目名 ----
    add(EntityType::Project, r"项目[：:]\s*[\u4e00-\u9fff]{2,10}");
    add(EntityType::Project, r"[\u4e00-\u9fff]{2,8}(?:计划|方案|工程|系统)(?:\s+v?[\d.]+)?");

    compiled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phone_scan() {
        let scanner = Scanner::new();
        let text = "联系人：张三，电话：13800138000";
        let matches = scanner.scan(text);
        assert!(matches.iter().any(|m| matches!(m.entity_type, EntityType::Phone)));
    }

    #[test]
    fn test_idcard_scan() {
        let scanner = Scanner::new();
        let text = "身份证号：110101199003074477";
        let matches = scanner.scan(text);
        assert!(matches.iter().any(|m| matches!(m.entity_type, EntityType::IdCard)));
    }

    #[test]
    fn test_email_scan() {
        let scanner = Scanner::new();
        let text = "邮箱：zhangsan@company.com";
        let matches = scanner.scan(text);
        assert!(matches.iter().any(|m| matches!(m.entity_type, EntityType::Email)));
    }

    #[test]
    fn test_region_scan() {
        let scanner = Scanner::new();
        let text = "审计组对河北省石家庄市新华区财政局开展了延伸审计";
        let matches = scanner.scan(text);
        assert!(matches.iter().any(|m| matches!(m.entity_type, EntityType::Region)),
            "应识别出地区名：{:?}", matches);
    }

    #[test]
    fn test_govdept_scan() {
        let scanner = Scanner::new();
        let text = "根据财政部和国家税务总局的要求，XX局开展了专项检查";
        let matches = scanner.scan(text);
        assert!(matches.iter().any(|m| matches!(m.entity_type, EntityType::GovDept)),
            "应识别出机关部门：{:?}", matches);
    }

    #[test]
    fn test_money_level() {
        // 金额应属于 Keep 级别
        assert_eq!(EntityType::Money.level(), SensitiveLevel::Keep);
        // 人名应属于 Mask 级别
        assert_eq!(EntityType::Person.level(), SensitiveLevel::Mask);
    }
}
