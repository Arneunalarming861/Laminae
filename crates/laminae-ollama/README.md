# laminae-ollama

Standalone Rust client for [Ollama](https://ollama.ai) — local LLM inference with zero cost, no API keys, no network egress.

Part of the [Laminae](https://github.com/Orellius/laminae) SDK.

## Features

- Blocking and streaming completions via `/api/chat`
- Multi-turn conversation support
- Model availability checking
- Configurable base URL and timeout
- Automatic retry on transient errors

## Quick Start

```rust
use laminae_ollama::OllamaClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = OllamaClient::new();

    if !client.is_available().await {
        eprintln!("Start Ollama with: ollama serve");
        return Ok(());
    }

    let response = client.complete(
        "llama3.2",
        "You are a helpful assistant.",
        "What is 2 + 2?",
        0.7,
        256,
    ).await?;

    println!("{response}");
    Ok(())
}
```

## Streaming

```rust
let mut rx = client.complete_streaming(
    "llama3.2", "You are helpful.", "Hello!", 0.7, 256,
).await?;

while let Some(chunk) = rx.recv().await {
    print!("{chunk}");
}
```

## Custom Configuration

```rust
use laminae_ollama::{OllamaClient, OllamaConfig};

let client = OllamaClient::with_config(OllamaConfig {
    base_url: "http://10.0.0.5:11434".to_string(),
    timeout_secs: 120,
});
```

## License

Apache-2.0 — see [LICENSE](../../LICENSE).
