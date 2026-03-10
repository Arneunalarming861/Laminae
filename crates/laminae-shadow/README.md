# laminae-shadow

Adversarial red-teaming engine that automatically audits AI output for security vulnerabilities. Runs as an async post-processing pipeline — never blocks the conversation.

Part of the [Laminae](https://github.com/Orellius/laminae) SDK.

## Pipeline Stages

1. **Static analysis** — Pattern scanning for 25+ vulnerability categories across multiple analyzers
2. **LLM adversarial review** — Local Ollama model with an attacker-mindset prompt
3. **Sandbox execution** — Ephemeral container testing (optional)

## Built-in Analyzers

| Analyzer | What It Catches |
|----------|----------------|
| `StaticAnalyzer` | SQL injection, XSS, eval/exec, path traversal, weak crypto, infinite loops, insecure deserialization |
| `SecretsAnalyzer` | GitHub tokens, Stripe keys, AWS keys, Slack tokens, JWTs, DB connection strings, API keys (10 patterns) |
| `DependencyAnalyzer` | Pipe-to-shell installs, insecure package indices, compromised NPM packages, git over HTTP |
| `LlmReviewer` | Free-form adversarial analysis via local LLM |

## Quick Start

```rust
use laminae_shadow::{ShadowEngine, ShadowEvent, create_report_store};

#[tokio::main]
async fn main() {
    let store = create_report_store();
    let engine = ShadowEngine::new(store.clone());

    let mut rx = engine.analyze_async(
        "session-1".into(),
        "```python\neval(user_input)\n```".into(),
    );

    while let Some(event) = rx.recv().await {
        match event {
            ShadowEvent::Finding { finding, .. } => {
                eprintln!("[{}] {}: {}",
                    finding.severity, finding.category, finding.title);
            }
            ShadowEvent::Done { report, .. } => {
                println!("Clean: {} | Issues: {}", report.clean, report.findings.len());
            }
            _ => {}
        }
    }
}
```

## Custom Analyzers

Implement the `Analyzer` trait to add your own analysis stages:

```rust
use laminae_shadow::analyzer::{Analyzer, AnalyzerError};
use laminae_shadow::extractor::ExtractedBlock;
use laminae_shadow::report::VulnFinding;

struct MyAnalyzer;

impl Analyzer for MyAnalyzer {
    fn name(&self) -> &'static str { "my-analyzer" }

    async fn is_available(&self) -> bool { true }

    async fn analyze(
        &self,
        ego_output: &str,
        code_blocks: &[ExtractedBlock],
    ) -> Result<Vec<VulnFinding>, AnalyzerError> {
        // Your analysis logic here
        Ok(vec![])
    }
}
```

## Configuration

```json
// ~/.config/laminae/shadow.json
{
    "enabled": true,
    "aggressiveness": 2,
    "llm_review_enabled": true,
    "shadow_model": "qwen2.5:7b",
    "sandbox_enabled": false
}
```

Aggressiveness levels:
- **1** — Static analysis only (no Ollama required)
- **2** — Static + LLM adversarial review
- **3** — Static + LLM + sandbox execution

## License

Apache-2.0 — see [LICENSE](../../LICENSE).
