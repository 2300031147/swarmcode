import { useState } from "react";
import { useFilesStore } from "@/store/files";
import { FileNode } from "@/lib/tauri";
import { fileIcon } from "@/lib/utils";

export default function FileTree() {
  const { tree, workspace } = useFilesStore();

  if (tree.length === 0) {
    return (
      <div style={{ padding: "12px 16px", color: "var(--text-2)", fontSize: 12 }}>
        {workspace ? "Empty folder" : "No folder opened"}
      </div>
    );
  }

  return (
    <div style={{ padding: "4px 0" }}>
      {tree.map((node) => (
        <FileNodeItem key={node.path} node={node} depth={0} />
      ))}
    </div>
  );
}

interface FileNodeItemProps {
  node: FileNode;
  depth: number;
}

function FileNodeItem({ node, depth }: FileNodeItemProps) {
  const [expanded, setExpanded] = useState(depth < 1);
  const { openFile, activeTab } = useFilesStore();

  const indent = depth * 12 + 8;
  const isActive = activeTab === node.path;

  const handleClick = () => {
    if (node.is_dir) {
      setExpanded((e) => !e);
    } else {
      openFile(node.path, node.name);
    }
  };

  return (
    <>
      <div
        className={`file-node ${node.is_dir ? "file-node-dir" : ""} ${isActive ? "active" : ""}`}
        style={{ paddingLeft: indent }}
        onClick={handleClick}
        title={node.path}
      >
        {/* Expand chevron for dirs */}
        {node.is_dir && (
          <span style={{ fontSize: 10, color: "var(--text-2)", width: 10, display: "inline-block" }}>
            {expanded ? "▾" : "▸"}
          </span>
        )}
        <span className="file-node-icon" style={{ fontSize: 14 }}>
          {node.is_dir
            ? expanded ? "📂" : "📁"
            : fileIcon(node.ext, false)}
        </span>
        <span className="file-node-name">{node.name}</span>
        {!node.is_dir && node.size !== null && (
          <span style={{ marginLeft: "auto", fontSize: 10, color: "var(--text-2)", paddingRight: 8 }}>
            {node.size < 1024
              ? `${node.size}B`
              : node.size < 1024 * 1024
              ? `${(node.size / 1024).toFixed(0)}KB`
              : `${(node.size / (1024 * 1024)).toFixed(1)}MB`}
          </span>
        )}
      </div>

      {/* Children */}
      {node.is_dir && expanded && node.children && (
        <div>
          {node.children.map((child) => (
            <FileNodeItem key={child.path} node={child} depth={depth + 1} />
          ))}
        </div>
      )}
    </>
  );
}
