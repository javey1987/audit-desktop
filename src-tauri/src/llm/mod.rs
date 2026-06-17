/// LLM 调用模块 — DeepSeek API 客户端

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChatMessage,
}

/// 聊天回复
#[derive(Debug, Serialize)]
pub struct LlmReply {
    pub content: String,
}

/// 调用 DeepSeek API（脱敏后的数据）
pub async fn chat(messages: Vec<(String, String)>, api_key: &str) -> Result<LlmReply, String> {
    let client = reqwest::Client::new();

    let chat_messages: Vec<ChatMessage> = messages
        .into_iter()
        .map(|(role, content)| ChatMessage { role, content })
        .collect();

    let request = ChatRequest {
        model: "deepseek-chat".into(),
        messages: chat_messages,
        stream: false,
    };

    let resp = client
        .post("https://api.deepseek.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("API 请求失败: {}", e))?;

    let body: ChatResponse = resp
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {}", e))?;

    let content = body
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_default();

    Ok(LlmReply { content })
}

/// 构建脱敏审计的 system prompt
pub fn build_system_prompt(file_info: &str, matched_columns: &str) -> String {
    format!(
        r#"你是审计砖家，一个专业的财务数据审计助手。

当前文件信息：
{file_info}

敏感字段识别结果：
{matched_columns}

【重要规则】
1. 你收到的数据已经过脱敏处理，人名、公司名等已被替换为占位符
2. 回复中请保持占位符不变，不要尝试猜测原始值
3. 专注于分析数据层面的异常（金额异常、重复支付、科目不匹配等）
4. 分析结果要用中文回复，清晰分点说明

请开始分析。"#
    )
}
