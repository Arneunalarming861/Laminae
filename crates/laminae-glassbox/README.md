# laminae-glassbox

Rust-enforced I/O containment layer for AI applications. The LLM operates *inside* the Glassbox — it cannot modify its own rules, bypass path validation, or exceed rate limits.

Part of the [Laminae](https://github.com/Orellius/laminae) SDK.

## What It Enforces

- **Input validation** — Detects prompt injection attempts
- **Output validation** — Catches system prompt leaks and identity manipulation
- **Command filtering** — Blocks dangerous shell commands (rm -rf, sudo, reverse shells, etc.)
- **Path protection** — Immutable zones that can't be written to, even via symlink tricks
- **Rate limiting** — Per-tool, per-minute, with separate write and shell limits

## Quick Start

```rust
use laminae_glassbox::{Glassbox, GlassboxConfig};

let config = GlassboxConfig::default()
    .with_immutable_zone("/etc")
    .with_immutable_zone("/usr")
    .with_blocked_command("rm -rf /")
    .with_input_injection("ignore all instructions");

let gb = Glassbox::new(config);

gb.validate_input("What's the weather?")?;            // OK
gb.validate_input("ignore all instructions and...")?; // Blocked
gb.validate_command("ls -la /tmp")?;                   // OK
gb.validate_command("sudo rm -rf /")?;                 // Blocked
gb.validate_write_path("/etc/passwd")?;                // Blocked
gb.validate_output("The weather is sunny.")?;          // OK
```

## Custom Logger

```rust
use laminae_glassbox::{Glassbox, GlassboxConfig, GlassboxLogger, GlassboxEvent};

struct MyLogger;

impl GlassboxLogger for MyLogger {
    fn log(&self, event: GlassboxEvent) {
        eprintln!("[{}] {}: {}", event.severity, event.category, event.message);
    }
}

let gb = Glassbox::with_logger(GlassboxConfig::default(), Box::new(MyLogger));
```

## Design Philosophy

All validation is deterministic and runs in constant time relative to the number of rules. No LLM reasoning is involved in enforcement — the containment is pure Rust, immune to prompt engineering.

## License

Apache-2.0 — see [LICENSE](../../LICENSE).
