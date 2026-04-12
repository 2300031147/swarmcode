use petgraph::graph::{NodeIndex, DiGraph};
use petgraph::Directed;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tree_sitter::{Language, Parser, Tree};

/// A metadata structure representing parsed AST information for a file.
#[derive(Debug, Clone)]
pub struct CodeNode {
    pub file_path: PathBuf,
    pub symbol_name: String,
    pub node_type: CodeNodeType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeNodeType {
    Function,
    Class,
    Struct,
    Module,
    Variable,
}

/// The Knowledge Engine Graph natively driven by petgraph.
/// Replaces the Python swarm-senses backend implementation.
pub struct CodeGraph {
    pub graph: DiGraph<CodeNode, String, u32>,
    pub node_map: HashMap<String, NodeIndex>,
}

impl CodeGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
        }
    }

    /// Add a parsed node to the graph and return its index
    pub fn add_node(&mut self, full_symbol: String, node: CodeNode) -> NodeIndex {
        if let Some(&idx) = self.node_map.get(&full_symbol) {
            return idx;
        }
        let idx = self.graph.add_node(node);
        self.node_map.insert(full_symbol, idx);
        idx
    }

    /// Link two AST nodes together indicating a semantic relationship
    pub fn add_dependency(&mut self, from_symbol: &str, to_symbol: &str, relation: &str) {
        if let (Some(&from_idx), Some(&to_idx)) = (self.node_map.get(from_symbol), self.node_map.get(to_symbol)) {
            self.graph.add_edge(from_idx, to_idx, relation.to_string());
        }
    }

    /// Search for symbols matching a keyword or exact name
    pub fn find_symbols(&self, query: &str) -> Vec<CodeNode> {
        self.node_map.iter()
            .filter(|(name, _)| name.to_lowercase().contains(&query.to_lowercase()))
            .filter_map(|(_, &idx)| self.graph.node_weight(idx).cloned())
            .collect()
    }
}

pub mod extractor;

/// Core function to bootstrap the knowledge graph indexing process.
/// In production, this binds to tree-sitter grammars for specific languages (e.g., Rust, Python).
pub fn initialize_swarm_senses() -> CodeGraph {
    println!("swarm-senses engine configured. Initializing blazing-fast AST Extraction...");
    let mut cg = CodeGraph::new();

    // Dynamically crawl the local workspace upon boot to extract ASTs, completely replacing Python!
    let ext = extractor::CodebaseExtractor::new();
    ext.extract_workspace(std::env::current_dir().unwrap_or_default().as_path(), &mut cg);

    // Map internal framework boundaries explicitly
    cg.add_dependency("ClawSwarm::swarm_matrix", "ClawSwarm::core", "imports");

    cg
}
