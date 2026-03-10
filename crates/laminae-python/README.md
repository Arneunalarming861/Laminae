# laminae-python

Python bindings for [Laminae](https://github.com/Orellius/laminae) via [PyO3](https://pyo3.rs). Exposes Glassbox (I/O containment), VoiceFilter (AI slop detection), and Cortex (edit tracking / learning loop) as native Python classes backed by Rust.

Part of the [Laminae](https://github.com/Orellius/laminae) SDK (v0.3).

## Installation

> PyPI package coming soon. For now, build from source with [maturin](https://www.maturin.rs/).

```bash
git clone https://github.com/orellius/laminae.git
cd laminae/crates/laminae-python
pip install maturin
maturin develop
```

## Glassbox -- I/O Containment

Validates inputs, outputs, commands, and file write paths. Raises `ValueError` on policy violations.

```python
from laminae import Glassbox

gb = Glassbox()

gb.validate_input("Hello")             # OK
gb.validate_output("The sky is blue.") # OK
gb.validate_command("rm -rf /")        # raises ValueError
gb.validate_write_path("/etc/passwd")  # raises ValueError
```

## VoiceFilter -- AI Slop Detection

Catches AI-sounding phrases, filler language, and stylistic violations.

```python
from laminae import VoiceFilter

f = VoiceFilter()
result = f.check("It's important to note that...")

print(result.passed)       # False
print(result.cleaned)      # cleaned version of the text
print(result.violations)   # ["AI vocabulary detected: ..."]
print(result.severity)     # 2
print(result.retry_hints)  # ["DO NOT use formal/academic language..."]
```

Configuration options:

```python
f = VoiceFilter(
    max_sentences=3,
    max_chars=280,
    reject_trailing_questions=True,
    fix_em_dashes=True,
)
```

## Cortex -- Edit Tracking

Tracks user edits to AI output, detects patterns, and generates learned prompt instructions.

```python
from laminae import Cortex

c = Cortex(min_edits=5)

# Track user edits (original -> edited)
c.track_edit("It's worth noting X.", "X.")
c.track_edit("Furthermore, Y is important.", "Y matters.")

# Detect patterns once enough edits are tracked
patterns = c.detect_patterns()
for p in patterns:
    print(f"{p.pattern_type}: {p.frequency_pct:.0f}%")

# Generate a prompt block from learned patterns
hints = c.get_prompt_block()

# View statistics
stats = c.stats()
print(f"Edit rate: {stats['edit_rate']:.0%}")
```

## Architecture

The Python module is a thin PyO3 wrapper around the Rust crates (`laminae-glassbox`, `laminae-persona`, `laminae-cortex`). All computation happens in compiled Rust. Python calls into native code with no interpreter overhead for the core logic.

## Docs

Full reference: [laminae.dev/reference/python-bindings](https://laminae.dev/reference/python-bindings)

## License

Apache-2.0 -- see [LICENSE](../../LICENSE).
