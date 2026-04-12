import { invoke } from "@tauri-apps/api/core";

// ── File System ────────────────────────────────────────────────────────────
export const fsListDir = (path: string) =>
  invoke<FileNode[]>("fs_list_dir", { path });

export const fsReadFile = (path: string) =>
  invoke<ReadFileResult>("fs_read_file", { path });

export const fsWriteFile = (path: string, content: string) =>
  invoke<void>("fs_write_file", { path, content });

export const fsCreateFile = (path: string) =>
  invoke<void>("fs_create_file", { path });

export const fsDeleteFile = (path: string) =>
  invoke<void>("fs_delete_file", { path });

export const fsRename = (from: string, to: string) =>
  invoke<void>("fs_rename", { from, to });

export const fsGetWorkspace = () =>
  invoke<string | null>("fs_get_workspace");

export const fsSetWorkspace = (path: string) =>
  invoke<FileNode[]>("fs_set_workspace", { path });

// ── Chat ───────────────────────────────────────────────────────────────────
export const chatSendMessage = (userMessage: string, model: string) =>
  invoke<ChatResponse>("chat_send_message", { userMessage, model });

export const chatGetModels = () =>
  invoke<ModelEntry[]>("chat_get_models");

export const chatClearHistory = () =>
  invoke<void>("chat_clear_history");

export const chatCompactHistory = () =>
  invoke<string>("chat_compact_history");

export const chatGetCost = () =>
  invoke<CostReport>("chat_get_cost");

// ── Providers ──────────────────────────────────────────────────────────────
export const providersList = () =>
  invoke<ProviderEntry[]>("providers_list");

export const providersTest = (name: string) =>
  invoke<ProviderTestResult>("providers_test", { name });

export const providersSetModel = (model: string) =>
  invoke<ModelSwitchResult>("providers_set_model", { model });

export const providersGetConfigPath = () =>
  invoke<string>("providers_get_config_path");

export const providersReadConfig = () =>
  invoke<string>("providers_read_config");

export const providersWriteConfig = (content: string) =>
  invoke<void>("providers_write_config", { content });

// ── Agents ─────────────────────────────────────────────────────────────────
export const agentsList = () =>
  invoke<HiveStatus>("agents_list");

export const agentsStartSwarm = () =>
  invoke<HiveStatus>("agents_start_swarm");

export const agentsSendMessage = (message: string, to?: string) =>
  invoke<string>("agents_send_message", { message, to });

export const agentsGetHiveStatus = () =>
  invoke<HiveStatus>("agents_get_hive_status");

// ── Terminal ───────────────────────────────────────────────────────────────
export const terminalRun = (command: string) =>
  invoke<TerminalOutput>("terminal_run", { command });

export const terminalRunInWorkspace = (command: string, workspace: string) =>
  invoke<TerminalOutput>("terminal_run_in_workspace", { command, workspace });

// ── App ────────────────────────────────────────────────────────────────────
export const appGetVersion = () =>
  invoke<AppInfo>("app_get_version");

// ── Shared Types ───────────────────────────────────────────────────────────
export interface FileNode {
  name: string;
  path: string;
  is_dir: boolean;
  ext: string | null;
  size: number | null;
  children: FileNode[] | null;
}

export interface ReadFileResult {
  path: string;
  content: string;
  language: string;
}

export interface ChatMessage {
  id: string;
  role: "user" | "assistant" | "system";
  content: string;
  model: string | null;
  tokens_in: number | null;
  tokens_out: number | null;
  timestamp_ms: number;
}

export interface ChatResponse {
  message: ChatMessage;
  total_cost_usd: number;
  total_input_tokens: number;
  total_output_tokens: number;
}

export interface CostReport {
  total_input_tokens: number;
  total_output_tokens: number;
  estimated_cost_usd: number;
  session_messages: number;
}

export interface ModelEntry {
  id: string;
  display_name: string;
  provider: string;
  is_local: boolean;
}

export interface ProviderEntry {
  name: string;
  kind: "local" | "hosted" | "custom";
  available: boolean;
  url: string;
  requires_key: boolean;
  key_set: boolean;
  models: string[];
}

export interface ProviderTestResult {
  name: string;
  reachable: boolean | null;
  url?: string;
  message?: string;
}

export interface ModelSwitchResult {
  model: string;
  provider: string;
  success: boolean;
}

export interface AgentEntry {
  id: string;
  name: string;
  description: string;
  role: string;
  status: string;
  last_seen_ms: number;
}

export interface HiveStatus {
  agents: AgentEntry[];
  total: number;
  active: number;
  idle: number;
}

export interface TerminalOutput {
  stdout: string;
  stderr: string;
  exit_code: number;
  success: boolean;
}

export interface AppInfo {
  version: string;
  name: string;
  tauri_version: string;
}
