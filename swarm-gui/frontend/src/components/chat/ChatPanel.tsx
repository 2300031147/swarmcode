import { useEffect, useRef, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { useChatStore } from "@/store/chat";
import { useFilesStore } from "@/store/files";
import { formatTokens, formatCost, formatTime } from "@/lib/utils";
import { ChatMessage } from "@/lib/tauri";

export default function ChatPanel() {
  const {
    messages, model, models, isLoading, cost,
    sendMessage, loadModels, setModel, clearHistory, compactHistory,
  } = useChatStore();
  const { activeTab, tabs } = useFilesStore();
  const [input, setInput] = useState("");
  const messagesEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    loadModels();
  }, []);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, isLoading]);

  const handleSend = async () => {
    const text = input.trim();
    if (!text || isLoading) return;
    setInput("");
    await sendMessage(text);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  // Inject active file context
  const injectFileContext = () => {
    if (!activeTab) return;
    const tab = tabs.find((t) => t.path === activeTab);
    if (tab) {
      setInput(
        `Here is the file \`${tab.name}\`:\n\`\`\`${tab.language}\n${tab.content.slice(0, 3000)}\n\`\`\`\n\n`
      );
    }
  };

  return (
    <div className="chat-panel" style={{ height: "100%" }}>
      {/* Header */}
      <div className="chat-header">
        <span className="chat-header-title">🤖 AI Chat</span>
        <select
          className="model-select"
          value={model}
          onChange={(e) => setModel(e.target.value)}
          title="Active model"
        >
          {models.length === 0 ? (
            <option value={model}>{model}</option>
          ) : (
            models.map((m) => (
              <option key={m.id} value={m.id}>
                {m.display_name}
              </option>
            ))
          )}
        </select>
        <button
          onClick={clearHistory}
          className="btn btn-ghost btn-sm"
          title="Clear history"
        >
          🗑
        </button>
      </div>

      {/* Messages */}
      <div className="chat-messages selectable">
        {messages.length === 0 && (
          <div style={{ textAlign: "center", color: "var(--text-2)", padding: "32px 12px" }}>
            <div style={{ fontSize: 32, marginBottom: 12 }}>💬</div>
            <div style={{ fontWeight: 600, marginBottom: 6 }}>ClawSwarm AI Ready</div>
            <div style={{ fontSize: 12 }}>
              Ask about your code, request changes, or run a swarm task.
            </div>
          </div>
        )}

        {messages.map((msg) => (
          <MessageBubble key={msg.id} message={msg} />
        ))}

        {isLoading && (
          <div className="chat-message assistant">
            <div className="chat-message-role">AI</div>
            <div className="chat-message-body" style={{ display: "flex", gap: 8, alignItems: "center" }}>
              <div className="spinner" />
              <span style={{ color: "var(--text-2)" }}>Thinking...</span>
            </div>
          </div>
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* Footer: cost bar + input */}
      <div className="chat-footer">
        {cost && (
          <div className="chat-cost-bar">
            <span>Tokens:</span>
            <span className="chat-cost-badge">
              ↑{formatTokens(cost.total_input_tokens)} ↓{formatTokens(cost.total_output_tokens)}
            </span>
            <span>Est:</span>
            <span className="chat-cost-badge">{formatCost(cost.estimated_cost_usd)}</span>
            <button
              onClick={() => compactHistory()}
              className="btn btn-ghost btn-sm"
              style={{ marginLeft: "auto", fontSize: 10 }}
              title="Compact history"
            >
              Compact
            </button>
          </div>
        )}

        {/* Inject file button */}
        {activeTab && (
          <button
            className="btn btn-ghost btn-sm"
            onClick={injectFileContext}
            style={{ fontSize: 11 }}
          >
            📎 Add current file
          </button>
        )}

        <div className="chat-input-row">
          <textarea
            className="chat-input"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Ask anything… (Enter to send, Shift+Enter for newline)"
            rows={1}
          />
          <button
            className="chat-send-btn"
            onClick={handleSend}
            disabled={isLoading || !input.trim()}
            title="Send (Enter)"
          >
            ➤
          </button>
        </div>
      </div>
    </div>
  );
}

// ── Message bubble ──────────────────────────────────────────────────────────
function MessageBubble({ message }: { message: ChatMessage }) {
  return (
    <div className={`chat-message ${message.role}`}>
      <div className="chat-message-role">
        {message.role === "user" ? "You" : "AI"}
      </div>
      <div className="chat-message-body selectable">
        {message.role === "assistant" ? (
          <ReactMarkdown remarkPlugins={[remarkGfm]}>
            {message.content}
          </ReactMarkdown>
        ) : (
          <pre style={{ whiteSpace: "pre-wrap", fontFamily: "var(--font-ui)", margin: 0 }}>
            {message.content}
          </pre>
        )}
      </div>
      <div className="chat-message-meta">
        <span>{formatTime(message.timestamp_ms)}</span>
        {message.model && <span>· {message.model}</span>}
        {message.tokens_out != null && (
          <span>· {formatTokens(message.tokens_out)} tokens</span>
        )}
      </div>
    </div>
  );
}
