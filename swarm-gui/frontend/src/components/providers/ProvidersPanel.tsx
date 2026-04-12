import { useEffect } from "react";
import { useProvidersStore } from "@/store/providers";
import { providersSetModel } from "@/lib/tauri";
import { useChatStore } from "@/store/chat";

export default function ProvidersPanel() {
  const { providers, testResults, isLoading, load, test } = useProvidersStore();
  const { setModel } = useChatStore();

  useEffect(() => {
    load();
  }, []);

  return (
    <div className="providers-panel">
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 4 }}>
        <span style={{ fontSize: 11, color: "var(--text-2)" }}>
          {providers.filter((p) => p.available).length}/{providers.length} available
        </span>
        <button className="btn btn-ghost btn-sm" onClick={load} disabled={isLoading}>
          {isLoading ? <span className="spinner" /> : "↻ Refresh"}
        </button>
      </div>

      {providers.map((p) => (
        <div key={p.name} className="provider-card">
          <div className="provider-card-header">
            <div className={`provider-dot ${p.available ? "online" : "offline"}`} />
            <span className="provider-name">{p.name}</span>
            <span className={`provider-badge ${p.kind}`}>{p.kind}</span>
          </div>

          <div className="provider-url">{p.url}</div>

          {p.requires_key && !p.key_set && (
            <div style={{ fontSize: 11, color: "var(--yellow)", marginTop: 2 }}>
              ⚠ API key not set
            </div>
          )}

          {/* Test result */}
          {testResults[p.name] && (
            <div style={{ fontSize: 11, marginTop: 2 }}>
              {testResults[p.name].reachable === true && (
                <span style={{ color: "var(--green)" }}>✓ Reachable</span>
              )}
              {testResults[p.name].reachable === false && (
                <span style={{ color: "var(--red)" }}>✗ Not reachable</span>
              )}
              {testResults[p.name].message && (
                <span style={{ color: "var(--text-2)" }}>
                  {" "}{testResults[p.name].message}
                </span>
              )}
            </div>
          )}

          {/* Model chips */}
          {p.models.length > 0 && (
            <div className="provider-models">
              {p.models.slice(0, 6).map((m) => (
                <span
                  key={m}
                  className="provider-model-chip"
                  onClick={async () => {
                    await providersSetModel(m);
                    setModel(m);
                  }}
                  title={`Use ${m}`}
                >
                  {m}
                </span>
              ))}
            </div>
          )}

          {/* Actions */}
          <div className="provider-actions">
            {(p.kind === "local") && (
              <button
                className="btn btn-ghost btn-sm"
                onClick={() => test(p.name)}
              >
                Test
              </button>
            )}
            {p.models[0] && (
              <button
                className="btn btn-primary btn-sm"
                onClick={() => setModel(p.models[0])}
              >
                Use
              </button>
            )}
          </div>
        </div>
      ))}

      <div style={{ fontSize: 11, color: "var(--text-2)", marginTop: 4 }}>
        Edit <code style={{ fontSize: 10 }}>~/.clawswarm/providers.toml</code> to add custom endpoints
      </div>
    </div>
  );
}
