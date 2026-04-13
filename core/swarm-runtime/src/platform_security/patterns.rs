use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;

/// Security threat patterns for command injection detection
/// Inspired by Goose Security Patterns
#[derive(Debug, Clone)]
pub struct ThreatPattern {
    pub name: &'static str,
    pub pattern: &'static str,
    pub description: &'static str,
    pub risk_level: RiskLevel,
    pub category: ThreatCategory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Low,      // Minor security issue
    Medium,   // Moderate security concern
    High,     // Significant security risk
    Critical, // Immediate system compromise risk
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreatCategory {
    FileSystemDestruction,
    RemoteCodeExecution,
    DataExfiltration,
    SystemModification,
    NetworkAccess,
    ProcessManipulation,
    PrivilegeEscalation,
    CommandInjection,
}

impl RiskLevel {
    pub fn confidence_score(&self) -> f32 {
        match self {
            RiskLevel::Critical => 0.95,
            RiskLevel::High => 0.75,
            RiskLevel::Medium => 0.60,
            RiskLevel::Low => 0.45,
        }
    }
}

/// Comprehensive list of dangerous command patterns ported from Goose
pub const THREAT_PATTERNS: &[ThreatPattern] = &[
    // Critical filesystem destruction patterns
    ThreatPattern {
        name: "rm_rf_root",
        pattern: r"rm\s+(-[rf]*[rf][rf]*|--recursive|--force).*[/\\]",
        description: "Recursive file deletion with rm -rf",
        risk_level: RiskLevel::High,
        category: ThreatCategory::FileSystemDestruction,
    },
    ThreatPattern {
        name: "rm_rf_system",
        pattern: r"rm\s+(-[rf]*[rf][rf]*|--recursive|--force).*(bin|etc|usr|var|sys|proc|dev|boot|lib|opt|srv|tmp)",
        description: "Recursive deletion of system directories",
        risk_level: RiskLevel::Critical,
        category: ThreatCategory::FileSystemDestruction,
    },
    ThreatPattern {
        name: "dd_destruction",
        pattern: r"dd\s+.*if=/dev/(zero|random|urandom).*of=/dev/[sh]d[a-z]",
        description: "Disk destruction using dd command",
        risk_level: RiskLevel::Critical,
        category: ThreatCategory::FileSystemDestruction,
    },
    ThreatPattern {
        name: "format_drive",
        pattern: r"(format|mkfs\.[a-z]+)\s+[/\\]dev[/\\][sh]d[a-z]",
        description: "Formatting system drives",
        risk_level: RiskLevel::Critical,
        category: ThreatCategory::FileSystemDestruction,
    },
    // Remote code execution patterns
    ThreatPattern {
        name: "curl_bash_execution",
        pattern: r"(curl|wget)\s+.*\|\s*(bash|sh|zsh|fish|csh|tcsh)",
        description: "Remote script execution via curl/wget piped to shell",
        risk_level: RiskLevel::Critical,
        category: ThreatCategory::RemoteCodeExecution,
    },
    ThreatPattern {
        name: "bash_process_substitution",
        pattern: r"bash\s*<\s*\(\s*(curl|wget)",
        description: "Bash process substitution with remote content",
        risk_level: RiskLevel::High,
        category: ThreatCategory::RemoteCodeExecution,
    },
    ThreatPattern {
        name: "python_remote_exec",
        pattern: r"python[23]?\s+-c\s+.*urllib|requests.*exec",
        description: "Python remote code execution",
        risk_level: RiskLevel::High,
        category: ThreatCategory::RemoteCodeExecution,
    },
    ThreatPattern {
        name: "powershell_download_exec",
        pattern: r"powershell.*DownloadString.*Invoke-Expression",
        description: "PowerShell remote script execution",
        risk_level: RiskLevel::High,
        category: ThreatCategory::RemoteCodeExecution,
    },
    // Data exfiltration patterns
    ThreatPattern {
        name: "ssh_key_exfiltration",
        pattern: r"(curl|wget).*-d.*\.ssh/(id_rsa|id_ed25519|id_ecdsa)",
        description: "SSH key exfiltration",
        risk_level: RiskLevel::High,
        category: ThreatCategory::DataExfiltration,
    },
    ThreatPattern {
        name: "credential_scanning",
        pattern: r"(grep|awk|sed).*-i.*(key|secret|password|token|auth|cred).*",
        description: "Scanning for credentials in local files",
        risk_level: RiskLevel::Medium,
        category: ThreatCategory::DataExfiltration,
    },
    ThreatPattern {
        name: "password_file_access",
        pattern: r"(cat|grep|awk|sed).*(/etc/passwd|/etc/shadow|\.password|\.env)",
        description: "Password file access",
        risk_level: RiskLevel::High,
        category: ThreatCategory::DataExfiltration,
    },
    ThreatPattern {
        name: "history_exfiltration",
        pattern: r"(curl|wget).*-d.*\.(bash_history|zsh_history|history)",
        description: "Command history exfiltration",
        risk_level: RiskLevel::High,
        category: ThreatCategory::DataExfiltration,
    },
    // System modification patterns
    ThreatPattern {
        name: "crontab_modification",
        pattern: r"(crontab\s+-e|echo.*>.*crontab|.*>\s*/var/spool/cron)",
        description: "Crontab modification for persistence",
        risk_level: RiskLevel::High,
        category: ThreatCategory::SystemModification,
    },
    ThreatPattern {
        name: "systemd_service_creation",
        pattern: r"systemctl.*enable|.*\.service.*>/etc/systemd",
        description: "Systemd service creation",
        risk_level: RiskLevel::High,
        category: ThreatCategory::SystemModification,
    },
    ThreatPattern {
        name: "hosts_file_modification",
        pattern: r"echo.*>.*(/etc/hosts|hosts\.txt)",
        description: "Hosts file modification",
        risk_level: RiskLevel::Medium,
        category: ThreatCategory::SystemModification,
    },
    // Network access patterns
    ThreatPattern {
        name: "netcat_listener",
        pattern: r"nc\s+(-l|-p)\s+\d+",
        description: "Netcat listener creation",
        risk_level: RiskLevel::High,
        category: ThreatCategory::NetworkAccess,
    },
    ThreatPattern {
        name: "reverse_shell",
        pattern: r"(nc|netcat|bash|sh).*-e\s*(bash|sh|/bin/bash|/bin/sh)",
        description: "Reverse shell creation",
        risk_level: RiskLevel::Critical,
        category: ThreatCategory::NetworkAccess,
    },
    ThreatPattern {
        name: "ssh_tunnel",
        pattern: r"ssh\s+.*-[LRD]\s+\d+:",
        description: "SSH tunnel creation",
        risk_level: RiskLevel::Medium,
        category: ThreatCategory::NetworkAccess,
    },
    // Process manipulation patterns
    ThreatPattern {
        name: "kill_security_process",
        pattern: r"kill(all)?\s+.*\b(antivirus|firewall|defender|security|monitor)\b",
        description: "Killing security processes",
        risk_level: RiskLevel::High,
        category: ThreatCategory::ProcessManipulation,
    },
    ThreatPattern {
        name: "process_injection",
        pattern: r"gdb\s+.*attach|ptrace.*PTRACE_POKETEXT",
        description: "Process injection techniques",
        risk_level: RiskLevel::High,
        category: ThreatCategory::ProcessManipulation,
    },
    // Privilege escalation patterns
    ThreatPattern {
        name: "sudo_without_password",
        pattern: r"echo.*NOPASSWD.*>.*sudoers",
        description: "Sudo privilege escalation",
        risk_level: RiskLevel::Critical,
        category: ThreatCategory::PrivilegeEscalation,
    },
    ThreatPattern {
        name: "suid_binary_creation",
        pattern: r"chmod\s+[47][0-7][0-7][0-7]|chmod\s+\+s",
        description: "SUID binary creation",
        risk_level: RiskLevel::High,
        category: ThreatCategory::PrivilegeEscalation,
    },
    // Command injection patterns
    ThreatPattern {
        name: "command_substitution",
        pattern: r"\$\([^)]*[;&|><][^)]*\)|`[^`]*[;&|><][^`]*`",
        description: "Command substitution with shell operators",
        risk_level: RiskLevel::High,
        category: ThreatCategory::CommandInjection,
    },
    ThreatPattern {
        name: "shell_metacharacters",
        pattern: r"[;&|`$]{1,2}\s*[a-zA-Z]+",
        description: "Potential shell injection characters followed by commands",
        risk_level: RiskLevel::Low,
        category: ThreatCategory::CommandInjection,
    },
    ThreatPattern {
        name: "encoded_commands",
        pattern: r"(base64|hex|url).*decode.*\|\s*(bash|sh)",
        description: "Encoded command execution",
        risk_level: RiskLevel::High,
        category: ThreatCategory::CommandInjection,
    },
    // Obfuscation and evasion patterns
    ThreatPattern {
        name: "base64_encoded_shell",
        pattern: r"(echo|printf)\s+[A-Za-z0-9+/=]{20,}\s*\|\s*base64\s+-d\s*\|\s*(bash|sh|zsh)",
        description: "Base64 encoded shell commands",
        risk_level: RiskLevel::High,
        category: ThreatCategory::CommandInjection,
    },
    ThreatPattern {
        name: "hex_encoded_commands",
        pattern: r"(echo|printf)\s+[0-9a-fA-F\\x]{20,}\s*\|\s*(xxd|od).*\|\s*(bash|sh)",
        description: "Hex encoded command execution",
        risk_level: RiskLevel::High,
        category: ThreatCategory::CommandInjection,
    },
];

lazy_static! {
    static ref COMPILED_PATTERNS: HashMap<&'static str, Regex> = {
        let mut patterns = HashMap::new();
        for threat in THREAT_PATTERNS {
            if let Ok(regex) = Regex::new(&format!("(?i){}", threat.pattern)) {
                patterns.insert(threat.name, regex);
            }
        }
        patterns
    };
}

pub struct PatternMatcher {
    patterns: &'static HashMap<&'static str, Regex>,
}

impl PatternMatcher {
    pub fn new() -> Self {
        Self {
            patterns: &COMPILED_PATTERNS,
        }
    }

    pub fn scan(&self, text: &str) -> Vec<PatternMatch> {
        let mut matches = Vec::new();
        for threat in THREAT_PATTERNS {
            if let Some(regex) = self.patterns.get(threat.name) {
                if regex.is_match(text) {
                    for regex_match in regex.find_iter(text) {
                        matches.push(PatternMatch {
                            threat: threat.clone(),
                            matched_text: regex_match.as_str().to_string(),
                            start: regex_match.start(),
                            end: regex_match.end(),
                        });
                    }
                }
            }
        }
        matches.sort_by_key(|m| (std::cmp::Reverse(m.threat.risk_level), m.start));
        matches
    }

    pub fn has_critical_threats(&self, matches: &[PatternMatch]) -> bool {
        matches.iter().any(|m| m.threat.risk_level >= RiskLevel::High)
    }
}

pub struct PatternMatch {
    pub threat: ThreatPattern,
    pub matched_text: String,
    pub start: usize,
    pub end: usize,
}
