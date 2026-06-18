/// 标记化引擎：扫描文本 → 替换敏感内容为标记 → 记录映射

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::scanner::{EntityType, MatchResult, Scanner};

/// 一条替换映射
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenEntry {
    pub placeholder: String,   // [PERSON_1]
    pub original: String,      // 张三
    pub entity_type: String,   // "Person"
    pub entity_label: String,  // "人名"
}

/// 完整映射表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMap {
    pub entries: Vec<TokenEntry>,
    /// 原始文本 → 脱敏文本 的映射表（快速查找用）
    #[serde(skip)]
    pub original_to_placeholder: HashMap<String, String>,
    /// 标记 → 原始文本
    #[serde(skip)]
    pub placeholder_to_original: HashMap<String, String>,
}

impl TokenMap {
    pub fn new() -> Self {
        TokenMap {
            entries: Vec::new(),
            original_to_placeholder: HashMap::new(),
            placeholder_to_original: HashMap::new(),
        }
    }

    /// 添加一条映射
    pub fn add(&mut self, placeholder: String, original: String, entity_type: &EntityType) {
        self.original_to_placeholder.insert(original.clone(), placeholder.clone());
        self.placeholder_to_original.insert(placeholder.clone(), original.clone());
        self.entries.push(TokenEntry {
            placeholder,
            original,
            entity_type: entity_type.prefix().to_string(),
            entity_label: entity_type.label().to_string(),
        });
    }

    /// 恢复文本中的标记
    pub fn restore(&self, text: &str) -> String {
        let mut result = text.to_string();
        // 按标记长度从长到短替换，避免短标记替换长标记的一部分
        let mut sorted: Vec<_> = self.placeholder_to_original.iter().collect();
        sorted.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        for (placeholder, original) in &sorted {
            result = result.replace(placeholder.as_str(), original.as_str());
        }
        result
    }

    /// 统计各类型数量
    pub fn stat(&self) -> HashMap<String, usize> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for entry in &self.entries {
            *counts.entry(entry.entity_type.clone()).or_insert(0) += 1;
        }
        counts
    }
}

/// 标记器
pub struct Tokenizer {
    scanner: Scanner,
    /// 每种实体的计数器
    counters: HashMap<String, u32>,
    /// 上次生成的映射表
    pub last_map: Option<TokenMap>,
}

impl Tokenizer {
    pub fn new() -> Self {
        Tokenizer {
            scanner: Scanner::new(),
            counters: HashMap::new(),
            last_map: None,
        }
    }

    /// 加载自定义敏感词表（YAML 格式内容）
    pub fn load_custom_dict(&mut self, dict: HashMap<String, Vec<String>>) {
        self.scanner.load_custom_dict(dict);
    }

    /// 标记化：扫描文本，替换为标记，返回脱敏文本和映射
    pub fn tokenize(&mut self, text: &str) -> (String, TokenMap) {
        let matches = self.scanner.scan(text);
        let mut map = TokenMap::new();
        self.counters.clear();

        // 去重（相同原文只生成一个标记）
        let mut seen_originals: HashMap<String, &MatchResult> = HashMap::new();
        for m in &matches {
            if !seen_originals.contains_key(&m.original) {
                seen_originals.insert(m.original.clone(), m);
            }
        }

        // 为每个唯一实体生成标记
        for (original, m) in &seen_originals {
            let prefix = m.entity_type.prefix();
            let counter = self.counters.entry(prefix.to_string()).or_insert(0);
            *counter += 1;
            let placeholder = format!("[{}_{}]", prefix, counter);
            map.add(placeholder, original.clone(), &m.entity_type);
        }

        // 替换：从长到短，避免部分替换
        let mut sanitized = text.to_string();
        let mut sorted_entries: Vec<_> = map.entries.iter().collect();
        sorted_entries.sort_by(|a, b| b.original.len().cmp(&a.original.len()));
        
        for entry in &sorted_entries {
            sanitized = sanitized.replace(&entry.original, &entry.placeholder);
        }

        self.last_map = Some(map.clone());
        (sanitized, map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_and_restore() {
        let mut tokenizer = Tokenizer::new();
        let text = "张三的电话是13800138000，邮箱是zhangsan@test.com";
        
        let (sanitized, map) = tokenizer.tokenize(text);
        
        // 验证脱敏后不包含敏感信息
        assert!(!sanitized.contains("13800138000"));
        assert!(!sanitized.contains("张三"));
        
        // 验证还原
        let restored = map.restore(&sanitized);
        assert_eq!(restored, text);
        
        // 验证标记格式
        assert!(sanitized.contains("[PERSON_1]"));
        assert!(sanitized.contains("[PHONE_1]"));
        assert!(sanitized.contains("[EMAIL_1]"));
        
        println!("原文本: {}", text);
        println!("脱敏后: {}", sanitized);
        println!("还原后: {}", restored);
    }

    #[test]
    fn test_dedup_same_value() {
        let mut tokenizer = Tokenizer::new();
        let text = "联系人张三，电话13800138000，负责人也是张三";
        
        let (sanitized, _) = tokenizer.tokenize(text);
        
        // 同一个"张三"应使用相同的标记
        let count_person = sanitized.matches("[PERSON_1]").count();
        assert_eq!(count_person, 2, "'张三'出现了两次，应使用同一个标记");
    }
}
