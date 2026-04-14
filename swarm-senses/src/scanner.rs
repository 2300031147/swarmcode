use std::path::Path;
use std::fs;
use ignore::WalkBuilder;
use tree_sitter::Parser;
use crate::{CodeGraph, CodeNode, CodeNodeType};
use tracing::{info, error};

pub struct CodebaseExtractor;

impl Default for CodebaseExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl CodebaseExtractor {
    pub fn new() -> Self {
        Self
    }
}

fn language_for_ext(ext: &str) -> Option<(tree_sitter::Language, &'static str)> {
    match ext {
        "rs" => Some((tree_sitter_rust::language(), "rust")),
        "py" => Some((tree_sitter_python::language(), "python")),
        "ts" => Some((tree_sitter_typescript::language_typescript(), "typescript")),
        "tsx" => Some((tree_sitter_typescript::language_tsx(), "typescript")),
        "js" | "jsx" | "mjs" => Some((tree_sitter_javascript::language(), "javascript")),
        "go" => Some((tree_sitter_go::language(), "go")),
        "java" => Some((tree_sitter_java::language(), "java")),
        "cpp" | "cc" | "cxx" | "hpp" => Some((tree_sitter_cpp::language(), "cpp")),
        _ => None,
    }
}

impl CodebaseExtractor {
    pub fn extract_workspace(&self, root_dir: &Path, graph: &mut CodeGraph) {
        info!("[swarm-senses: Extractor] Initiating high-speed parallel file crawl targeting {:?}", root_dir);

        let walker = WalkBuilder::new(root_dir)
            .hidden(true)
            .git_ignore(true)
            .build();

        let mut files_scanned = 0;

        for result in walker {
            match result {
                Ok(entry) => {
                    if entry.path().is_file() {
                        let ext = entry.path().extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("");
                        
                        if let Some((lang, lang_name)) = language_for_ext(ext) {
                            let mut parser = Parser::new();
                            if let Err(e) = parser.set_language(&lang) {
                                error!("ERROR: Grammar load failure for {}: {}", lang_name, e);
                                continue;
                            }
                            self.parse_file(root_dir, entry.path(), graph, &mut parser, lang_name);
                            files_scanned += 1;
                        }
                    }
                }
                Err(err) => error!("ERROR: Failed to walk file: {}", err),
            }
        }
        
        info!("[swarm-senses: Extractor] File trace exhaustive. Total source files indexed: {}", files_scanned);
    }

    fn parse_file(&self, root_dir: &Path, path: &Path, graph: &mut CodeGraph, parser: &mut Parser, lang: &str) {
        let relative_path = path.strip_prefix(root_dir).unwrap_or(path);
        let module_symbol = format!("module::{}", relative_path.display()).replace('\\', "/");
        
        // Push the Root Module Node
        let _node_idx = graph.add_node(module_symbol.clone(), CodeNode {
            file_path: path.to_path_buf(),
            symbol_name: module_symbol.clone(),
            node_type: CodeNodeType::Module,
        });

        // 1. Read file natively into bytes
        if let Ok(source_code) = fs::read(path) {
            if let Some(tree) = parser.parse(&source_code, None) {
                self.walk_node(path, &module_symbol, tree.root_node(), graph, &source_code, lang);
            }
        }
    }

    fn walk_node(&self, path: &Path, module_symbol: &str, node: tree_sitter::Node, graph: &mut CodeGraph, source_code: &[u8], lang: &str) {
        let kind = node.kind();
        let mut child_module_symbol = module_symbol.to_string();

        let (node_type, name_field) = match (lang, kind) {
            // Rust
            ("rust", "function_item") => (Some(CodeNodeType::Function), Some("name")),
            ("rust", "struct_item")   => (Some(CodeNodeType::Struct),   Some("name")),
            ("rust", "impl_item")     => (Some(CodeNodeType::Class),    Some("type")),
            ("rust", "trait_item")    => (Some(CodeNodeType::Class),    Some("name")),
            // Python
            ("python", "function_definition") => (Some(CodeNodeType::Function), Some("name")),
            ("python", "class_definition")    => (Some(CodeNodeType::Class),    Some("name")),
            // TypeScript / JavaScript
            ("typescript"|"javascript", "function_declaration") => (Some(CodeNodeType::Function), Some("name")),
            ("typescript"|"javascript", "class_declaration")    => (Some(CodeNodeType::Class),    Some("name")),
            ("typescript"|"javascript", "method_definition")     => (Some(CodeNodeType::Function), Some("name")),
            ("typescript", "interface_declaration") => (Some(CodeNodeType::Class), Some("name")),
            // Go
            ("go", "function_declaration") => (Some(CodeNodeType::Function), Some("name")),
            ("go", "type_declaration")     => (Some(CodeNodeType::Struct),   Some("name")),
            ("go", "method_declaration")   => (Some(CodeNodeType::Function), Some("name")),
            // Java
            ("java", "method_declaration") => (Some(CodeNodeType::Function), Some("name")),
            ("java", "class_declaration")  => (Some(CodeNodeType::Class),    Some("name")),
            // C++
            ("cpp", "function_definition") => (Some(CodeNodeType::Function), Some("name")),
            ("cpp", "class_specifier")     => (Some(CodeNodeType::Class),    Some("name")),
            ("cpp", "struct_specifier")    => (Some(CodeNodeType::Struct),   Some("name")),
            _ => (None, None),
        };

        if let (Some(nt), Some(nf)) = (node_type, name_field) {
            if let Some(name_node) = node.child_by_field_name(nf) {
                if let Ok(symbol_name) = std::str::from_utf8(&source_code[name_node.start_byte()..name_node.end_byte()]) {
                    let symbol_name = symbol_name.trim();
                    let symbol_fqn = format!("{}::{}", module_symbol, symbol_name);
                    
                    graph.add_node(symbol_fqn.clone(), CodeNode {
                        file_path: path.to_path_buf(),
                        symbol_name: symbol_fqn.clone(),
                        node_type: nt,
                    });

                    graph.add_dependency(module_symbol, &symbol_fqn, "contains");
                    child_module_symbol = symbol_fqn; // Update for recursion
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.walk_node(path, &child_module_symbol, child, graph, source_code, lang);
        }
    }
        }
    }
}
