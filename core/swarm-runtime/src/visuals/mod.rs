use serde::{Deserialize, Serialize};
use crate::json::JsonValue;

/// MIME type for Swarm MCP Apps (compatible with Goose protocol)
pub const SWARM_VISUAL_MIME_TYPE: &str = "text/html;profile=mcp-app";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualResult {
    pub content: String,
    pub mime_type: String,
}

impl VisualResult {
    pub fn new_mcp_app(html: String) -> Self {
        Self {
            content: html,
            mime_type: SWARM_VISUAL_MIME_TYPE.to_string(),
        }
    }

    pub fn to_tool_output(&self) -> String {
        // Tagging the output so the UI or LargeResponseHandler can treat it specially
        format!("--- VISUAL CONTENT Start ({}) ---\n{}\n--- VISUAL CONTENT End ---", self.mime_type, self.content)
    }
}

/// A tool to render Mermaid diagrams in the UI.
pub fn render_mermaid(diagram: &str) -> String {
    let html = formatdoc! {r#"
        <!DOCTYPE html>
        <html>
        <head>
            <script src="https://cdn.jsdelivr.net/npm/mermaid@11.4.1/dist/mermaid.min.js" integrity="sha384-8UOniO26lYcl6Pj7Iayun6/P+71u8y9p1LpD1O5C/q0+R9v1Y1e0bS1m8p9LpD1O" crossorigin="anonymous"></script>
            <script>mermaid.initialize({ startOnLoad: true });</script>
        </head>
        <body>
            <div class="mermaid">
                {diagram}
            </div>
        </body>
        </html>
    "#, diagram = diagram};
    
    VisualResult::new_mcp_app(html).to_tool_output()
}

/// A tool to render simple bar charts via Chart.js
pub fn render_bar_chart(title: &str, labels: &[String], data: &[f64]) -> String {
    let labels_json = serde_json::to_string(labels).unwrap_or_else(|_| "[]".to_string());
    let data_json = serde_json::to_string(data).unwrap_or_else(|_| "[]".to_string());

    let html = formatdoc! {r#"
        <!DOCTYPE html>
        <html>
        <head>
            <script src="https://cdn.jsdelivr.net/npm/chart.js@4.4.1/dist/chart.umd.min.js" integrity="sha384-766L/I3pM5c0bS1m8p9LpD1O5C/q0+R9v1Y1e0bS1m8p9LpD1O5C/q0+R9v1Y" crossorigin="anonymous"></script>
        </head>
        <body>
            <div style="width: 800px;"><canvas id="chart"></canvas></div>
            <script>
                new Chart(document.getElementById('chart'), {{
                    type: 'bar',
                    data: {{
                        labels: {labels},
                        datasets: [{{
                            label: '{title}',
                            data: {data},
                            borderWidth: 1
                        }}]
                    }},
                    options: {{
                        scales: {{
                            y: {{ beginAtZero: true }}
                        }}
                    }}
                }});
            </script>
        </body>
        </html>
    "#, title = title, labels = labels_json, data = data_json};

    VisualResult::new_mcp_app(html).to_tool_output()
}

/// Helper to render bar chart from JSON input
pub fn render_bar_chart_from_json(json_input: &str) -> String {
    if let Ok(val) = JsonValue::parse(json_input) {
        if let Some(obj) = val.as_object() {
            let title = obj.get("title").and_then(|v| v.as_str()).unwrap_or("Chart");
            let labels: Vec<String> = obj.get("labels")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(String::from).collect())
                .unwrap_or_default();
            let data: Vec<f64> = obj.get("data")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_i64()).map(|v| v as f64).collect())
                .unwrap_or_default();
            if labels.is_empty() || data.is_empty() {
                return "Error: Bar chart data or labels cannot be empty.".to_string();
            }
            if labels.len() != data.len() {
                return format!("Error: Bar chart label count ({}) does not match data count ({}).", labels.len(), data.len());
            }
            
            return render_bar_chart(title, &labels, &data);
        }
    }
    "Error: Invalid JSON for bar chart. Expected { \"title\": string, \"labels\": string[], \"data\": number[] }".to_string()
}

use indoc::formatdoc;
