use chromiumoxide::Page;
use serde::Serialize;
use std::error::Error;

pub struct RefEngine;

#[derive(Debug, Serialize)]
pub struct TaggedElement {
    pub r#ref: String,
    pub tag: String,
    pub r#type: String,
    pub text: String,
    pub href: String,
    pub placeholder: String,
}

impl RefEngine {
    pub fn new() -> Self {
        Self
    }

    /// Inject data-ClawSwarm-ref IDs on all interactive elements.
    pub async fn inject_dom_references(&self, page: &Page) -> Result<(), Box<dyn Error>> {
        let script = r#"
            (() => {
                let id = 0;
                const sel = 'button,a,input,textarea,select,[role="button"],[role="link"],[role="menuitem"],[contenteditable="true"]';
                document.querySelectorAll(sel).forEach(el => {
                    if (!el.hasAttribute('data-ClawSwarm-ref')) {
                        el.setAttribute('data-ClawSwarm-ref', String(id));
                        id++;
                    }
                });
                return id;
            })();
        "#;

        let count = page.evaluate(script).await?
            .value()
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        println!("[RefEngine] Tagged {count} interactive elements.");
        Ok(())
    }

    /// Extract a compact JSON array of all tagged elements — what the LLM reasons about.
    pub async fn extract_tagged_elements(
        &self,
        page: &Page,
    ) -> Result<Vec<TaggedElement>, Box<dyn Error>> {
        let script = r#"
            (() => {
                return Array.from(document.querySelectorAll('[data-ClawSwarm-ref]')).map(el => ({
                    ref: el.getAttribute('data-ClawSwarm-ref') || '',
                    tag: el.tagName.toLowerCase(),
                    type: el.type || el.getAttribute('type') || '',
                    text: (el.innerText || el.textContent || el.value || '').trim().slice(0, 120),
                    href: el.href || el.getAttribute('href') || '',
                    placeholder: el.placeholder || el.getAttribute('placeholder') || ''
                }));
            })();
        "#;

        let value = page.evaluate(script).await?
            .value()
            .cloned()
            .unwrap_or(serde_json::Value::Array(vec![]));

        let elements: Vec<TaggedElement> = serde_json::from_value(value)
            .unwrap_or_default();

        Ok(elements)
    }

    /// Extract visible page text for the final DOM snapshot.
    pub async fn extract_page_text(&self, page: &Page) -> Result<String, Box<dyn Error>> {
        let script = r#"
            (() => {
                const title = document.title;
                const body = (document.body?.innerText || '').trim().slice(0, 4000);
                return `[${title}]\n${body}`;
            })();
        "#;

        let text = page.evaluate(script).await?
            .value()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default();

        Ok(text)
    }

    /// Click element by ref ID.
    pub async fn click_element_by_ref(
        &self,
        page: &Page,
        ref_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        let sanitized = sanitize_ref_id(ref_id);
        let selector = format!("[data-ClawSwarm-ref='{sanitized}']");
        let element = page.find_element(&selector).await?;
        element.click().await?;
        Ok(())
    }

    /// Type text into element by ref ID.
    pub async fn type_into_element_by_ref(
        &self,
        page: &Page,
        ref_id: &str,
        text: &str,
    ) -> Result<(), Box<dyn Error>> {
        let sanitized = sanitize_ref_id(ref_id);
        let selector = format!("[data-ClawSwarm-ref='{sanitized}']");
        let element = page.find_element(&selector).await?;
        element.click().await?;
        element.type_str(text).await?;
        Ok(())
    }
}

impl Default for RefEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn sanitize_ref_id(id: &str) -> String {
    id.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(32)
        .collect()
}
