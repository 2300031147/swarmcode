import { getCurrentWindow } from "@tauri-apps/api/window";

interface TitleBarProps {
  workspace: string | null;
  onOpenFolder: () => void;
}

export default function TitleBar({ workspace, onOpenFolder }: TitleBarProps) {
  const appWindow = getCurrentWindow();

  return (
    <div className="titlebar">
      {/* Window control buttons */}
      <div className="titlebar-controls">
        <button className="titlebar-btn close"  onClick={() => appWindow.close()} title="Close" />
        <button className="titlebar-btn min"    onClick={() => appWindow.minimize()} title="Minimize" />
        <button className="titlebar-btn max"    onClick={() => appWindow.toggleMaximize()} title="Maximize" />
      </div>

      {/* Logo */}
      <div className="titlebar-logo">
        <span className="text-sm font-medium text-gray-400">SwarmCode</span>
      </div>

      {/* Current workspace path */}
      <div className="titlebar-path">
        {workspace ?? "No workspace open"}
      </div>

      {/* Actions */}
      <div className="titlebar-actions">
        <button className="btn btn-ghost btn-sm" onClick={onOpenFolder} title="Open Folder">
          📂 Open
        </button>
      </div>
    </div>
  );
}
