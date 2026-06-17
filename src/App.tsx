import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { FileInfo, DesensitizeResult, ColumnSummary } from "./lib/types";

function App() {
  const [filePath, setFilePath] = useState("");
  const [fileInfo, setFileInfo] = useState<FileInfo | null>(null);
  const [data, setData] = useState<DesensitizeResult | null>(null);
  const [message, setMessage] = useState("");
  const [chatLog, setChatLog] = useState<{ role: string; content: string }[]>([]);
  const [loading, setLoading] = useState(false);
  const [apiKey, setApiKey] = useState("");

  // 选择文件
  async function pickFile() {
    const selected = await open({
      multiple: false,
      filters: [{ name: "表格文件", extensions: ["xlsx", "csv"] }],
    });
    if (selected) {
      setFilePath(selected);
      setLoading(true);
      try {
        const info: FileInfo = await invoke("open_excel", { path: selected });
        setFileInfo(info);
        const result: DesensitizeResult = await invoke("desensitize_file", { path: selected });
        setData(result);
      } catch (e) {
        alert("打开文件失败: " + e);
      }
      setLoading(false);
    }
  }

  // 发送消息
  async function sendMessage() {
    if (!message.trim() || !filePath) return;
    setChatLog((prev) => [...prev, { role: "user", content: message }]);
    setLoading(true);
    try {
      const result: { reply: string } = await invoke("chat_with_llm", {
        message,
        filePath,
      });
      setChatLog((prev) => [...prev, { role: "assistant", content: result.reply }]);
    } catch (e) {
      setChatLog((prev) => [...prev, { role: "assistant", content: "❌ " + e }]);
    }
    setMessage("");
    setLoading(false);
  }

  // 敏感列颜色
  function colColor(type: string): string {
    const colors: Record<string, string> = {
      PERSON: "bg-red-100 text-red-800 border-red-200",
      COMPANY: "bg-orange-100 text-orange-800 border-orange-200",
      PHONE: "bg-yellow-100 text-yellow-800 border-yellow-200",
      ID_CARD: "bg-purple-100 text-purple-800 border-purple-200",
      BANK_CARD: "bg-pink-100 text-pink-800 border-pink-200",
      EMAIL: "bg-teal-100 text-teal-800 border-teal-200",
      ADDR: "bg-cyan-100 text-cyan-800 border-cyan-200",
      MONEY: "bg-green-100 text-green-800 border-green-200",
    };
    return colors[type] || "bg-gray-100 text-gray-600 border-gray-200";
  }

  return (
    <div className="h-screen flex flex-col">
      {/* 顶部栏 */}
      <header className="px-6 py-4 border-b flex items-center justify-between"
        style={{ borderColor: "var(--muted)", background: "var(--subtle)" }}>
        <div className="flex items-center gap-3">
          <h1 className="text-xl font-bold tracking-tight" style={{ color: "var(--accent)" }}>
            审计砖家
          </h1>
          <span className="text-sm opacity-50">桌面版</span>
        </div>
        <div className="flex items-center gap-3">
          <input
            type="password"
            placeholder="DeepSeek API Key"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            className="px-3 py-1.5 text-sm rounded border bg-white"
            style={{ borderColor: "var(--muted)" }}
          />
          <button
            onClick={pickFile}
            className="px-4 py-1.5 rounded text-sm font-medium text-white"
            style={{ background: "var(--accent)" }}
          >
            打开文件
          </button>
        </div>
      </header>

      {/* 主体 */}
      <div className="flex flex-1 overflow-hidden">
        {/* 左侧：文件预览 */}
        <div className="w-1/2 flex flex-col border-r" style={{ borderColor: "var(--muted)" }}>
          {data ? (
            <>
              <div className="p-3 text-sm border-b" style={{ borderColor: "var(--muted)" }}>
                共 {data.rows.length} 行 · 识别到 {data.matched_count} 列敏感字段
              </div>
              <div className="flex-1 overflow-auto p-2">
                <table className="w-full text-xs border-collapse">
                  <thead>
                    <tr>
                      <th className="px-2 py-1 text-left border-b sticky top-0 bg-white">#</th>
                      {data.headers.map((h, i) => (
                        <th key={i} className={`px-2 py-1 border-b sticky top-0 bg-white ${colColor(data.columns[i]?.sensitive_type)}`}>
                          {h}
                          {data.columns[i]?.sensitive_label && (
                            <span className="ml-1 text-[10px] opacity-60">
                              ({data.columns[i].sensitive_type})
                            </span>
                          )}
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {data.rows.slice(0, 100).map((row, ri) => (
                      <tr key={ri} className="hover:bg-gray-50">
                        <td className="px-2 py-0.5 border-b text-gray-400">{ri + 1}</td>
                        {row.map((cell, ci) => (
                          <td key={ci} className={`px-2 py-0.5 border-b max-w-[200px] truncate ${
                            cell.startsWith("[") && cell.endsWith("]") ? "font-mono text-orange-600" : ""
                          }`}>
                            {cell}
                          </td>
                        ))}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </>
          ) : (
            <div className="flex-1 flex items-center justify-center text-sm opacity-40">
              点击"打开文件"选择 .xlsx 或 .csv
            </div>
          )}
        </div>

        {/* 右侧：对话 */}
        <div className="w-1/2 flex flex-col">
          <div className="flex-1 overflow-auto p-4 space-y-3">
            {chatLog.map((msg, i) => (
              <div key={i} className={`flex ${msg.role === "user" ? "justify-end" : "justify-start"}`}>
                <div className={`max-w-[80%] rounded-lg px-4 py-2 text-sm ${
                  msg.role === "user"
                    ? "text-white"
                    : "border"
                }`}
                  style={{
                    background: msg.role === "user" ? "var(--accent)" : undefined,
                    borderColor: msg.role === "user" ? "var(--accent)" : "var(--muted)",
                  }}>
                  <pre className="whitespace-pre-wrap font-sans">{msg.content}</pre>
                </div>
              </div>
            ))}
            {loading && (
              <div className="flex justify-start">
                <div className="border rounded-lg px-4 py-2 text-sm animate-pulse"
                  style={{ borderColor: "var(--muted)" }}>
                  分析中...
                </div>
              </div>
            )}
          </div>
          <div className="p-3 border-t" style={{ borderColor: "var(--muted)" }}>
            <div className="flex gap-2">
              <input
                value={message}
                onChange={(e) => setMessage(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && sendMessage()}
                placeholder="输入分析指令..."
                disabled={!filePath || loading}
                className="flex-1 px-3 py-2 text-sm rounded border"
                style={{ borderColor: "var(--muted)" }}
              />
              <button
                onClick={sendMessage}
                disabled={!filePath || loading || !message.trim()}
                className="px-4 py-2 rounded text-sm font-medium text-white disabled:opacity-40"
                style={{ background: "var(--accent)" }}
              >
                发送
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

export default App;
