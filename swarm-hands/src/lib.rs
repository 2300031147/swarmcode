pub mod browser;

use chromiumoxide::{Browser, BrowserConfig, Page};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::PathBuf;
use std::sync::OnceLock;
use tokio::time::{sleep, Duration};

static CHROMIUM_PATH: OnceLock<PathBuf> = OnceLock::new();

// ── Max safety iterations ─────────────────────────────────────────────────
const MAX_ITERATIONS: usize = 15;

// ── Action protocol ───────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "lowercase")]
enum AgentAction {
    Click { r#ref: String },
    Type { r#ref: String, text: String },
    Navigate { url: String },
    Scroll { direction: String },
    Wait { ms: Option<u64> },
    Done { result: String },
}

// ── DOM element snapshot ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct TaggedElement {
    r#ref: String,
    tag: String,
    r#type: String,
    text: String,
    href: String,
    placeholder: String,
}

// ── WebAgent ──────────────────────────────────────────────────────────────

pub struct WebAgent {
    pub current_url: String,
    pub dom_snapshot: String,
    pub last_result: String,
}

impl WebAgent {
    pub fn new() -> Self {
        Self {
            current_url: String::from("about:blank"),
            dom_snapshot: String::new(),
            last_result: String::new(),
        }
    }

    pub fn is_active(&self) -> bool {
        true
    }

    pub async fn execute_task(
        &mut self,
        url: &str,
        task: &str,
        show_browser: bool,
    ) -> Result<(), Box<dyn Error>> {
        println!("🚀 SwarmHands: launching browser for: {}", url);

        // ── Chromium path (cached after first fetch) ──────────────────────
        let chromium_path = if let Some(path) = CHROMIUM_PATH.get() {
            path.clone()
        } else {
            println!(">> Fetching Chromium (first run only)...");
            let fetcher = chromiumoxide::fetcher::BrowserFetcher::new(
                chromiumoxide::fetcher::BrowserFetcherOptions::builder()
                    .build()
                    .unwrap(),
            );
            let info = fetcher
                .fetch()
                .await
                .map_err(|e| format!("Browser fetch failed: {e}"))?;
            let path = info.executable_path;
            CHROMIUM_PATH.set(path.clone()).ok();
            path
        };

        // ── Launch ────────────────────────────────────────────────────────
        let mut builder = BrowserConfig::builder()
            .window_size(1440, 900)
            .chrome_executable(chromium_path);

        if show_browser {
            builder = builder.with_head();
        }

        let (mut browser, mut handler) = Browser::launch(builder.build()?).await?;

        let cdp_handle = tokio::task::spawn(async move {
            while let Some(h) = handler.next().await {
                if h.is_err() {
                    break;
                }
            }
        });

        // ── Navigate ──────────────────────────────────────────────────────
        println!(">> Navigating to {url}...");
        let page = browser.new_page(url).await?;
        self.current_url = url.to_string();

        // Wait for page to settle
        sleep(Duration::from_millis(800)).await;

        // ── Initial DOM snapshot ──────────────────────────────────────────
        let ref_engine = crate::browser::RefEngine::new();
        ref_engine.inject_dom_references(&page).await?;
        self.dom_snapshot = ref_engine.extract_page_text(&page).await?;

        // ── Agent loop ────────────────────────────────────────────────────
        self.run_engineering_loop(&page, &ref_engine, task).await?;

        // ── Cleanup ───────────────────────────────────────────────────────
        browser.close().await?;
        let _ = cdp_handle.await;
        Ok(())
    }

    async fn run_engineering_loop(
        &mut self,
        page: &Page,
        ref_engine: &crate::browser::RefEngine,
        task: &str,
    ) -> Result<(), Box<dyn Error>> {
        let model = std::env::var("CLAWSWARM_BROWSER_MODEL")
            .unwrap_or_else(|_| std::env::var("CLAWSWARM_DEFAULT_MODEL")
                .unwrap_or_else(|_| "llama3.2".to_string()));

        println!("🤖 Agent loop started — model: {model} — task: {task}");

        for iteration in 1..=MAX_ITERATIONS {
            println!("\n── Iteration {iteration}/{MAX_ITERATIONS} ──");

            // 1. Extract tagged elements from live DOM
            let elements = ref_engine.extract_tagged_elements(page).await
                .unwrap_or_default();

            if elements.is_empty() {
                // Re-inject refs if the page navigated and lost them
                ref_engine.inject_dom_references(page).await?;
            }

            let elements_json = serde_json::to_string_pretty(&elements)
                .unwrap_or_else(|_| "[]".to_string());

            // 2. Build prompt
            let prompt = build_agent_prompt(task, &self.current_url, &elements_json, iteration);

            // 3. Call LLM
            println!(">> Calling model...");
            let raw_response = match call_ollama(&model, &prompt).await {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("LLM call failed: {e}. Retrying with simpler prompt.");
                    let simple = format!(
                        "Task: {task}\nURL: {}\nRespond with JSON only: {{\"action\":\"done\",\"result\":\"Could not complete task\"}}",
                        self.current_url
                    );
                    call_ollama(&model, &simple).await
                        .unwrap_or_else(|_| r#"{"action":"done","result":"LLM unavailable"}"#.to_string())
                }
            };

            println!(">> LLM response: {}", raw_response.trim());

            // 4. Parse action
            let action = match parse_action(&raw_response) {
                Some(a) => a,
                None => {
                    eprintln!("Could not parse action from response. Skipping iteration.");
                    continue;
                }
            };

            // 5. Execute action
            match action {
                AgentAction::Done { result } => {
                    println!("✅ Task complete: {result}");
                    self.last_result = result;
                    // Final DOM snapshot
                    self.dom_snapshot = ref_engine.extract_page_text(page).await
                        .unwrap_or_default();
                    return Ok(());
                }

                AgentAction::Click { r#ref: ref_id } => {
                    println!(">> Click ref={ref_id}");
                    if let Err(e) = ref_engine.click_element_by_ref(page, &ref_id).await {
                        eprintln!("Click failed: {e}");
                    }
                    sleep(Duration::from_millis(600)).await;
                    // Re-inject after potential DOM change
                    ref_engine.inject_dom_references(page).await.ok();
                }

                AgentAction::Type { r#ref: ref_id, text } => {
                    println!(">> Type ref={ref_id} text={:?}", &text);
                    if let Err(e) = ref_engine.type_into_element_by_ref(page, &ref_id, &text).await {
                        eprintln!("Type failed: {e}");
                    }
                    sleep(Duration::from_millis(300)).await;
                }

                AgentAction::Navigate { url } => {
                    println!(">> Navigate to {url}");
                    if let Err(e) = page.goto(&url).await {
                        eprintln!("Navigate failed: {e}");
                    } else {
                        self.current_url = url;
                        sleep(Duration::from_millis(1000)).await;
                        ref_engine.inject_dom_references(page).await.ok();
                    }
                }

                AgentAction::Scroll { direction } => {
                    println!(">> Scroll {direction}");
                    let script = if direction == "up" {
                        "window.scrollBy(0, -600);"
                    } else {
                        "window.scrollBy(0, 600);"
                    };
                    page.evaluate(script).await.ok();
                    sleep(Duration::from_millis(400)).await;
                    ref_engine.inject_dom_references(page).await.ok();
                }

                AgentAction::Wait { ms } => {
                    let wait_ms = ms.unwrap_or(1000).min(5000);
                    println!(">> Wait {wait_ms}ms");
                    sleep(Duration::from_millis(wait_ms)).await;
                }
            }

            // 6. Update URL after each action (may have changed)
            if let Ok(url_val) = page.evaluate("window.location.href").await {
                if let Some(url_str) = url_val.value().and_then(|v| v.as_str().map(String::from)) {
                    if url_str != self.current_url {
                        println!(">> URL changed to {url_str}");
                        self.current_url = url_str;
                    }
                }
            }
        }

        println!("⚠️  Max iterations reached without Done action.");
        self.last_result = format!(
            "Reached max iterations ({MAX_ITERATIONS}) without completing task: {task}"
        );
        self.dom_snapshot = ref_engine.extract_page_text(page).await
            .unwrap_or_default();
        Ok(())
    }
}

impl Default for WebAgent {
    fn default() -> Self {
        Self::new()
    }
}

// ── Prompt builder ────────────────────────────────────────────────────────

fn build_agent_prompt(
    task: &str,
    url: &str,
    elements_json: &str,
    iteration: usize,
) -> String {
    format!(
        r#"You are a browser automation agent. Complete the task by choosing ONE action.

TASK: {task}
CURRENT URL: {url}
ITERATION: {iteration}

INTERACTIVE ELEMENTS ON PAGE:
{elements_json}

AVAILABLE ACTIONS (respond with exactly one JSON object, no markdown, no explanation):
{{"action":"click","ref":"N"}}           — click element with ref N
{{"action":"type","ref":"N","text":"..."}} — type into element with ref N
{{"action":"navigate","url":"..."}}        — go to a URL
{{"action":"scroll","direction":"down"}}   — scroll down (or "up")
{{"action":"wait","ms":1000}}              — wait milliseconds
{{"action":"done","result":"..."}}         — task complete, describe what you found/did

Rules:
- If the task requires filling a form, type into the field first, then click submit.
- If you navigated to the right page and found the answer, use "done".
- If there are no relevant elements, try navigate or scroll.
- Respond with ONLY the JSON object."#
    )
}

// ── Ollama HTTP client ────────────────────────────────────────────────────

async fn call_ollama(model: &str, prompt: &str) -> Result<String, Box<dyn Error>> {
    let base_url = std::env::var("CLAWSWARM_OLLAMA_URL")
        .unwrap_or_else(|_| "http://localhost:11434".to_string());

    // Strip /v1 suffix if present — we use the native Ollama generate endpoint
    let base_url = base_url.trim_end_matches("/v1").to_string();

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120).into())
        .build()?;

    #[derive(Serialize)]
    struct OllamaRequest<'a> {
        model: &'a str,
        prompt: &'a str,
        stream: bool,
        format: &'a str,
    }

    #[derive(Deserialize)]
    struct OllamaResponse {
        response: String,
    }

    let resp = client
        .post(format!("{base_url}/api/generate"))
        .json(&OllamaRequest {
            model,
            prompt,
            stream: false,
            format: "json",
        })
        .send()
        .await?
        .json::<OllamaResponse>()
        .await?;

    Ok(resp.response.trim().to_string())
}

// ── Action parser ─────────────────────────────────────────────────────────

fn parse_action(raw: &str) -> Option<AgentAction> {
    // Strip markdown fences if present
    let cleaned = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // Try direct parse first
    if let Ok(action) = serde_json::from_str::<AgentAction>(cleaned) {
        return Some(action);
    }

    // Try to extract first JSON object from the text
    if let Some(start) = cleaned.find('{') {
        if let Some(end) = cleaned.rfind('}') {
            let slice = &cleaned[start..=end];
            if let Ok(action) = serde_json::from_str::<AgentAction>(slice) {
                return Some(action);
            }
        }
    }

    eprintln!("parse_action: could not parse: {cleaned:?}");
    None
}

pub fn initialize_daemon() {
    println!("Hands Browser Daemon Subsystem Booting...");
}
