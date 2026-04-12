import { useEffect, useState } from "react";
import { useAgentsStore } from "@/store/agents";
import { invoke } from "@tauri-apps/api/core";

export default function AgentsPanel() {
  const { hive, isLoading, swarmActive, load, startSwarm, sendMessage } = useAgentsStore();
  const [broadcastMsg, setBroadcastMsg] = useState("");
  
  // Browser Agent state
  const [browserUrl, setBrowserUrl] = useState("https://google.com");
  const [browserTask, setBrowserTask] = useState("");
  const [showBrowser, setShowBrowser] = useState(false);
  const [isBrowserRunning, setIsBrowserRunning] = useState(false);

  useEffect(() => {
    load();
    // Poll hive status every 5s while panel is visible
    const interval = setInterval(load, 5000);
    return () => clearInterval(interval);
  }, []);

  const handleBroadcast = async () => {
    if (!broadcastMsg.trim()) return;
    await sendMessage(broadcastMsg);
    setBroadcastMsg("");
  };

  const handleRunBrowserAgent = async () => {
    if (!browserTask.trim()) return;
    setIsBrowserRunning(true);
    try {
      await invoke("hands_run_agent", {
        req: {
          url: browserUrl,
          task: browserTask,
          show_browser: showBrowser
        }
      });
    } catch (err) {
      console.error("Browser Agent failed:", err);
    } finally {
      setIsBrowserRunning(false);
    }
  };

  return (
    <div className="agents-panel">
      {/* Hive Summary */}
      {hive && (
        <div className="hive-summary">
          <div className="hive-stat">
            <div className="hive-stat-value">{hive.total}</div>
            <div className="hive-stat-label">Total</div>
          </div>
          <div className="hive-stat">
            <div className="hive-stat-value" style={{ color: "var(--green)" }}>{hive.active}</div>
            <div className="hive-stat-label">Active</div>
          </div>
          <div className="hive-stat">
            <div className="hive-stat-value" style={{ color: "var(--text-2)" }}>{hive.idle}</div>
            <div className="hive-stat-label">Idle</div>
          </div>

          {/* Swarm button */}
          <button
            className={`btn btn-sm ${swarmActive ? "btn-ghost" : "btn-primary"}`}
            onClick={startSwarm}
            disabled={isLoading}
            style={{ marginLeft: "auto" }}
          >
            {isLoading ? <span className="spinner" /> : swarmActive ? "🔄 Refresh" : "🚀 Start Swarm"}
          </button>
        </div>
      )}

      {/* Browser Agent Controller */}
      <div className="agent-card" style={{ border: "1px solid #00e5ff33", background: "#00e5ff05" }}>
        <div className="agent-card-header">
          <span className="agent-name" style={{ color: "#00e5ff" }}>🌐 SwarmHands Agent</span>
          <div className="flex items-center gap-2">
            <span className="text-[10px] text-gray-400">Visible</span>
            <input 
              type="checkbox" 
              checked={showBrowser} 
              onChange={(e) => setShowBrowser(e.target.checked)}
              className="w-3 h-3 accent-[#00e5ff]"
            />
          </div>
        </div>
        <div className="flex flex-col gap-2 mt-2">
          <input
            className="chat-input"
            value={browserUrl}
            onChange={(e) => setBrowserUrl(e.target.value)}
            placeholder="Browser URL..."
            style={{ fontSize: 11, minHeight: 28 }}
          />
          <input
            className="chat-input"
            value={browserTask}
            onChange={(e) => setBrowserTask(e.target.value)}
            placeholder="Agent Task (e.g. Find pricing)..."
            style={{ fontSize: 11, minHeight: 28 }}
          />
          <button 
            className="btn btn-primary btn-sm" 
            onClick={handleRunBrowserAgent}
            disabled={isBrowserRunning || !browserTask}
          >
            {isBrowserRunning ? "Running..." : "Run Browser Agent"}
          </button>
        </div>
      </div>

      {/* Agent Cards */}
      {hive?.agents.map((agent) => (
        <div key={agent.id} className="agent-card">
          <div className="agent-card-header">
            <span className="agent-name">{agent.name}</span>
            <span className={`agent-status ${agent.status}`}>{agent.status}</span>
          </div>
          <div className="agent-description">{agent.description}</div>
          <div className="agent-role" style={{ marginTop: 4 }}>{agent.role}</div>
        </div>
      ))}

      {!hive && (
        <div style={{ textAlign: "center", color: "var(--text-2)", padding: "24px 12px" }}>
          <div style={{ fontSize: 32, marginBottom: 8 }}>🤖</div>
          <div style={{ marginBottom: 12 }}>No agents registered</div>
          <button className="btn btn-primary btn-sm" onClick={startSwarm}>
            🚀 Start Engineering Swarm
          </button>
        </div>
      )}

      {/* Broadcast message */}
      {hive && hive.agents.length > 0 && (
        <div style={{ display: "flex", gap: 6, marginTop: 4 }}>
          <input
            className="chat-input"
            style={{ flex: 1, minHeight: 32, fontSize: 12 }}
            value={broadcastMsg}
            onChange={(e) => setBroadcastMsg(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleBroadcast()}
            placeholder="Broadcast to all agents..."
          />
          <button
            className="btn btn-primary btn-sm"
            onClick={handleBroadcast}
            disabled={!broadcastMsg.trim()}
          >
            ➤
          </button>
        </div>
      )}
    </div>
  );
}
