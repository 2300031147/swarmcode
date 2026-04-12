use std::path::Path;
use std::fs;
use ignore::WalkBuilder;
use tree_sitter::Parser;
use crate::{CodeGraph, CodeNode, CodeNodeType};

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

    /// Recursively walks the directory and extracts AST nodes into the CodeGraph
    pub fn extract_workspace(&self, root_dir: &Path, graph: &mut CodeGraph) {
        println!("[swarm-senses: Extractor] Initiating high-speed parallel file crawl targeting {:?}", root_dir);

        let walker = WalkBuilder::new(root_dir)
            .hidden(true)
            .git_ignore(true)
            .build();

        let mut files_scanned = 0;
        let mut tree_parser = Parser::new();
        // Initialize the native C TreeSitter language grammar!
        tree_parser.set_language(&tree_sitter_rust::LANGUAGE.into()).expect("Error loading Rust grammar");

        for result in walker {
            match result {
                Ok(entry) => {
                    if entry.path().is_file() {
                        if let Some(ext) = entry.path().extension() {
                            if ext == "rs" {
                                self.parse_file(entry.path(), graph, &mut tree_parser);
                                files_scanned += 1;
                            }
                        }
                    }
                }
                Err(err) => println!("ERROR: Failed to walk file: {}", err),
            }
        }
        
        println!("[swarm-senses: Extractor] File trace exhaustive. Total Rust source files indexed: {}", files_scanned);
    }

    fn parse_file(&self, path: &Path, graph: &mut CodeGraph, parser: &mut Parser) {
        let module_symbol = format!("module::{}", path.file_stem().unwrap_or_default().to_string_lossy());
        
        // Push the Root Module Node
        let _node_idx = graph.add_node(module_symbol.clone(), CodeNode {
            file_path: path.to_path_buf(),
            symbol_name: module_symbol.clone(),
            node_type: CodeNodeType::Module,
        });

        // 1. Read file natively into bytes
        if let Ok(source_code) = fs::read(path) {
            // 2. Transcribe python `tree.parse()` into Rust tree_sitter invocation!
            if let Some(tree) = parser.parse(&source_code, None) {
                let root_node = tree.root_node();
                
                // 3. Recursive walk looking for Functions, Structs, Impls, and Traits
                let mut cursor = root_node.walk();
                for child in root_node.children(&mut cursor) {
                    let kind = child.kind();
                    if kind == "function_item" || kind == "struct_item" || kind == "impl_item" || kind == "trait_item" {
                        // Extract symbol name
                        let name_node_opt = if kind == "impl_item" {
                            child.child_by_field_name("type")
                        } else {
                            child.child_by_field_name("name")
                        };

                        if let Some(name_node) = name_node_opt {
                            if let Ok(symbol_name) = std::str::from_utf8(&source_code[name_node.start_byte()..name_node.end_byte()]) {
                                let symbol_fqn = format!("{}::{}", module_symbol, symbol_name);
                                
                                let node_type = match kind {
                                    "struct_item" => CodeNodeType::Struct,
                                    "impl_item" => CodeNodeType::Class, // Treating impl as Class mapping
                                    _ => CodeNodeType::Function,
                                };

                                graph.add_node(symbol_fqn.clone(), CodeNode {
                                    file_path: path.to_path_buf(),
                                    symbol_name: symbol_fqn.clone(),
                                    node_type,
                                });

                                // Construct edge in Petgraph
                                graph.add_dependency(&module_symbol, &symbol_fqn, "contains");
                            }
                        }
                    }
                }
            }
        }
    }
}
