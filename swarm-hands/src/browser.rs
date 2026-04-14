use chromiumoxide::Page;
use std::error::Error;

pub struct RefEngine;

impl RefEngine {
    pub fn new() -> Self {
        Self
    }

    /// Rewrites the TypeScript inject_refs.ts DOM mutation script into a native Rust evaluation that
    /// marks interactive elements with uniquely identifiable `[ref="123"]` tags for the Agent to interact with.
    pub async fn inject_dom_references(&self, page: &Page) -> Result<(), Box<dyn Error>> {
        println!("[Hands: RefEngine] Injecting semantic interaction boundaries into the DOM natively...");
        
        let extraction_script = r#"
            (() => {
                let id = 0;
                document.querySelectorAll('button, a, input, textarea, select, [role="button"]').forEach(el => {
                    if (!el.hasAttribute('data-ClawSwarm-ref')) {
                        el.setAttribute('data-ClawSwarm-ref', id.toString());
                        el.style.border = '2px solid red'; // Visual cue for headless viewing
                        id++;
                    }
                });
                return id;
            })();
        "#;

        let total_injected: i32 = page.evaluate(extraction_script)
            .await?
            .value()
            .unwrap_or_else(|| serde_json::json!(0))
            .as_i64()
            .unwrap_or(0) as i32;

        println!("[Hands: RefEngine] DOM modification complete. {} elements tagged for structured interaction.", total_injected);
        Ok(())
    }

    /// Executes a click on an element tagged by `inject_dom_references` using the CDP `page.click()` equivalent
    pub async fn click_element_by_ref(&self, page: &Page, ref_id: &str) -> Result<(), Box<dyn Error>> {
        let sanitized_id = ref_id.replace(['\'', '\"', '(', ')', '[', ']', '>', '<', '*', '|', '\\', '{', '}', ';', ':'], "");
        let selector = format!("[data-ClawSwarm-ref='{}']", sanitized_id);
        println!("[Hands: RefEngine] Emulating precise CDP click on selector: {}", selector);
        
        let element = page.find_element(selector.as_str()).await?;
        element.click().await?;
        
        Ok(())
    }

    /// Emulates keyboard typing into an element tagged by `inject_dom_references`
    pub async fn type_into_element_by_ref(&self, page: &Page, ref_id: &str, text: &str) -> Result<(), Box<dyn Error>> {
        let sanitized_id = ref_id.replace(['\'', '\"', '(', ')', '[', ']', '>', '<', '*', '|', '\\', '{', '}', ';', ':'], "");
        let selector = format!("[data-ClawSwarm-ref='{}']", sanitized_id);
        println!("[Hands: RefEngine] Emulating typing into selector: {} -> \"{}\"", selector, text);
        
        let element = page.find_element(selector.as_str()).await?;
        element.click().await?; // Focus the element
        element.type_str(text).await?;
        
        Ok(())
    }
}
