<div align="center">

```text
██╗  ██╗██████╗  █████╗ ██╗██╗   ██╗ ██████╗ ██████╗ ██████╗ ███████╗
██║ ██╔╝██╔══██╗██╔══██╗██║██║   ██║██╔════╝██╔═══██╗██╔══██╗██╔════╝
█████╔╝ ██████╔╝███████║██║██║   ██║██║     ██║   ██║██║  ██║█████╗
██╔═██╗ ██╔══██╗██╔══██║██║╚██╗ ██╔╝██║     ██║   ██║██║  ██║██╔══╝
██║  ██╗██║  ██║██║  ██║██║ ╚████╔╝ ╚██████╗╚██████╔╝██████╔╝███████╗
╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═══╝   ╚═════╝ ╚═════╝ ╚═════╝ ╚══════╝
```

[![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Platforms](https://img.shields.io/badge/platforms-Linux%20%7C%20macOS%20%7C%20Windows-blue?style=flat-square)](#platform-support)
[![Last Commit](https://badgen.net/github/last-commit/nkswalih/kraivcode/dev?icon=github)](https://github.com/nkswalih/kraivcode/commits/dev)
[![GitHub Stars](https://badgen.net/github/stars/nkswalih/kraivcode?icon=github)](https://github.com/nkswalih/kraivcode/stargazers)

A coding agent that asks before it assumes <br>
A coding agent that cannot touch your files until you say so

[Why](#why-kraivcode) · [Personas](#agent-personas) · [Plan mode](#plan-mode) · [Interface](#interface) · [Performance](#performance--resource-efficiency) · [Providers](#oauth-and-providers) · [Install](#installation) · [Quick start](#quick-start) · [Docs](#further-reading) · [Contributing](CONTRIBUTING.md)

</div>

---

<div align="left">

## Why Kraivcode

</div>

Most coding agents have committed to a turn before they know enough to take it.
They end with *"Want me to apply this?"* and stop — which reads like a question
and behaves like a dead end. You re-type your answer, they burn a turn, and
nothing in the tooling ever made the exchange safe.

Kraivcode is built around fixing exactly that. Two ideas:

**The agent should be able to ask.** `ask_user` opens a popup, blocks the turn,
and returns your answer to the same turn. Not a queued message — the turn that
asked the question is the turn that receives the answer.

**The agent should be incapable of ignoring you.** Switch persona to `Plan` and
write and execution tools are *removed from the tool list sent to the model*.
There is no instruction to disobey, no boundary to test.

<div align="left">

| | Kraivcode | A typical agent |
|---|---|---|
| Ending a turn with a question | Blocks the turn, opens a popup, resumes with your answer | Prints prose and stops |
| Read-only mode | Tool allowlist — write/exec tools are never sent | A system-prompt request to "not edit things" |
| Recovering a forgotten question | The daemon re-asks any trailing `?` it detects | Nothing happens |
| Switching operating mode | `Tab` — persisted across sessions | New prompt, new session |

</div>

---

<div align="center">

## Installation

</div>

Kraivcode does not publish prebuilt binaries yet, so it builds from source.

```bash
# macOS, Linux and Windows (any shell)
git clone https://github.com/nkswalih/kraivcode.git
cd kraivcode
cargo build --release --bin kraivcode

# the binary lands here
./target/release/kraivcode --version
```

Put it on your `PATH`. The installed command keeps the name `jcode` for compatibility with the on-disk layout and the updater:

```bash
# macOS & Linux
install -m 755 target/release/kraivcode ~/.local/bin/jcode
```

```powershell
# Windows 11 (PowerShell 5.1+)
New-Item -ItemType Directory -Force "$env:LOCALAPPDATA\kraivcode\bin" | Out-Null
Copy-Item target\release\kraivcode.exe "$env:LOCALAPPDATA\kraivcode\bin\jcode.exe"
```

**Requirements:** Rust 1.85+ (the workspace uses edition 2024), Git, and a
working linker. On Windows install Visual Studio Build Tools with the
**Desktop development with C++** workload.

Need a faster build loop, or want an agent to set it up for you?
[Jump to detailed installation](#detailed-installation).

### Updating

Run `/update` in the TUI to download the latest binary in the background and
reload with your session preserved. From a terminal, use `jcode update`,
then restart the client.

Older or equal versions are skipped. For a development build, Kraivcode also
compares the running binary's Git commit with the release tag: builds ahead of,
identical to, or diverged from the release are preserved. If ancestry cannot be
verified, the update stops rather than risking a downgrade.

`/rebuild` runs `git pull --ff-only` and a release build in the background while
you keep working, then reloads automatically. `/reload` swaps in a newer binary
without rebuilding, and `/restart` restarts on the current binary with the
session preserved.

### Where your data lives

Kraivcode keeps the engine's on-disk layout, so existing installs, migrations,
and session history carry over unchanged.

<div align="left">

| | |
|---|---|
| Config | `~/.jcode/config.toml` |
| Auth | `~/.jcode/auth.json` plus per-provider files |
| Sessions | `~/.jcode/sessions/` |
| Logs | `~/.jcode/logs/` |
| Memory | under `~/.jcode/` |
| MCP config | `~/.jcode/mcp.json`, `.jcode/mcp.json` |
| Environment overrides | `JCODE_*` (for example `JCODE_NO_EMOJI=1`) |
| Persona selection | `agent_persona.json` in the shared app config directory |

</div>

---

<div align="left">

## Agent Personas

</div>

<kbd>Tab</kbd> cycles forward, <kbd>Shift+Tab</kbd> backward. The active persona
is sent with **every** turn, so the daemon enforces policy instead of the client
merely suggesting it. Your selection persists to `agent_persona.json`, so it
survives restarts and follows you across attached clients.

<div align="left">

| Persona | Role |
|---|---|
| **Build** | Default. Full tool surface. |
| **Plan** | Read-only. Researches, asks, and produces a structured plan card. |
| **Swarm** | Coordinator for parallel worker fan-out. |
| **Review** | Headed reviewer session. DMs the parent session when it finishes. |
| **Judge** | Completion manager. Tells the parent to continue with concrete next steps, or that it is fine to stop. |
| **Memory** | Background memory extraction and consolidation role. |
| **Ambient** | Long-running unsupervised role. |

</div>

---

<div align="left">

## Plan Mode

</div>

### Read-only is structural, not advisory

Under the `Plan` persona the daemon filters the tool list sent to the provider
before the request leaves the process. The allowlist keeps:

<div align="left">

| Category | Tools |
|---|---|
| Read files | `read` `glob` `ls` |
| Search | `agentgrep` `session_search` `conversation_search` |
| Research | `webfetch` `websearch` `jcode_docs` |
| Memory | `memory` |
| Scratchpad | `side_panel` `todo` |
| Scheduling | `bg` `initiative` `schedule` |
| MCP management | `mcp` |
| Ask you | `ask_user` |

</div>

Write and execution tools are not sent at all. The model cannot call `write` or
`bash` because it has never been told they exist: there is no instruction to
ignore, no guardrail to route around, and no mistake to apologise for
afterwards. The daemon re-checks the same allowlist at execution time as
defense in depth.

That has a useful side effect. A Plan turn cannot shell out to a side effect,
install a package, or write to a secret path. Read-only means read-only, all the
way down.

### `ask_user` — the turn actually waits

`ask_user` is a real tool. The model calls it, a popup opens in your client, and
the turn stays alive until you reply.

<div align="left">

| | |
|---|---|
| **Options** | The model supplies concrete choices, for example `["Rename the flag", "Keep it", "Defer"]` |
| **Free text** | Always available unless the model sets `free_text = false` |
| **Answer handling** | Returns into the same turn — not queued as a new message |
| **Headless** | Returns a clear error instead of hanging. A question nobody can answer is reported, not deadlocked |

</div>

### The daemon re-asks anyway

Plan turns are also scanned for a trailing prose question. If a turn finishes
with an open `?` instead of calling `ask_user`, the daemon synthesises the same
popup so the question still gets answered. Trailing snippets are capped at 280
characters and answered with **Continue**.

### `/plan`

`/plan` is the conversational entry point. The agent inspects the repo,
investigates with read-only tools, and renders a plan card with **Goal**,
**Scope**, **Approach**, **Validation**, and **Open questions** — then stops.
Once you approve, it converts the plan into a todo list and starts the work.
`/plan` with no argument plans whatever is currently in focus.

---

<div align="left">

## Interface

</div>

<div align="left">

| Element | Behaviour |
|---|---|
| **Header** | One line. `KRAIVCODE` on the left, `project (branch)` dimmed on the right. No version banner, no provider matrix, no star bar |
| **Prompt gutter** | A fixed `┃ ` prefix, two columns wide. No prompt numbers, no reflow as the conversation grows |
| **Intent panel** | Borderless todo/worker overlay in the top right of the chat viewport, clamped between 24 and 60 columns and 2/5 of the width. Renders nothing at all once every todo is complete and no worker is running |
| **Status line** | Model, provider, and effort live at the bottom, where they do not push the conversation around when they change |
| **Selection** | Transcript drag-to-select copies on release and keeps the highlight, so a successful drag never looks like a failed one. Composer drags select without copying, so assembling a fragment can never wipe your clipboard |

</div>

### Rendering

Kraivcode renders at over a thousand frames per second. Your monitor will not
keep up, which is the point: there is no flicker to notice.

Scrollback is implemented in-app rather than delegated to the terminal, which is
what makes edge auto-scroll during a drag selection possible. Because a custom
scrollback cannot scroll a partial line at the terminal level, smooth half-line
scrolling needs a terminal that exposes a native scroll API.

### Side panels and diagrams

The `panel` tool opens a side panel from Markdown content or a linked
Markdown/PDF file, with update, focus, close, and list actions. Tell the agent
to load a file into the panel and watch it update live, or write directly into
it, or use it as a diff viewer. Both the panel and the chat render **mermaid**
diagrams inline, through a browser-free, TypeScript-free renderer.

### Alignment and emoji

Left-aligned by default. <kbd>Alt</kbd>+<kbd>C</kbd>, the `/alignment` command,
or config switches to centred. Set `emoji = false` under `[display]`, or launch
with `JCODE_NO_EMOJI=1`, to swap emoji for compact ASCII markers while leaving
the rest of Unicode intact.

---

<div align="left">

## Paste and Clipboard

</div>

Terminal paste is where most TUI agents fall over. Kraivcode handles the cases
that actually break in the field.

<div align="left">

| Case | Handling |
|---|---|
| **Bracketed paste** | Multi-line text arrives as one event and lands in the composer intact |
| **The stray trailing Enter** | Windows Terminal and legacy conhost deliver one extra bare Enter after a paste ending in a newline. A 150 ms window swallows that Enter and nothing else |
| **No bracketed paste at all** | Legacy conhost, some SSH setups, some tmux configs deliver a paste as raw injected keys, one Enter per `\r`. Kraivcode classifies by timing, not by guessing |
| **Images** | Paste an image with nothing selected and it is probed on the system clipboard and attached as image content |

</div>

**Enter storms.** Injection floods events with sub-millisecond gaps, while even
very fast typists stay above ~30 ms between presses. Events arriving within
12 ms of each other, with at least two inside a 150 ms window, mark a burst —
and an <kbd>Enter</kbd> landing inside that burst is treated as injected rather
than typed. No heuristic on the text content, no configuration toggle.

**Images.** Supported paths include `wl-paste`, an `<img src>` URL found in
pasted HTML, macOS via `osascript`, and `arboard` for anything that exposes
image data directly. Browser "copy image" is handled specially: the clipboard
usually carries both the URL and the pixels, and the in-clipboard image wins,
because re-downloading from a hotlink-protected host often fails. Empty text
never wins over an image — on Wayland and `arboard`, an image-only clipboard
frequently advertises an empty text target, which used to produce a silent
zero-character paste.

---

<div align="left">

## Skills Browser

</div>

`/skills` opens a searchable overlay instead of a wall of text. It lists loaded
skills — <kbd>Enter</kbd> activates one — alongside endorsed skills you do not
have installed yet, where <kbd>Enter</kbd> copies the install command or source.
It mirrors the account picker: centred box, filter row on top, list/detail split
in the middle, category-group navigation on <kbd>Left</kbd>/<kbd>Right</kbd>,
and the hotkey row drawn into the bottom border.

The old plain-text report is still reachable at `/skills-text`.

---

<div align="center">

## Performance & Resource Efficiency

</div>

Every session is a thin client attached to one long-lived daemon, so the
marginal cost of another session is single-digit megabytes rather than another
full agent runtime. Here is the shape of that under fan-out:

<div align="center">

<table>
  <tr>
    <td valign="top" align="center" width="50%">
      <strong>1 active session</strong>
      <table>
        <thead>
          <tr>
            <th>Tool</th>
            <th>PSS</th>
            <th>Comparison</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td><strong>Kraivcode</strong></td>
            <td align="right"><strong>32.6 MB</strong></td>
            <td align="right">baseline</td>
          </tr>
          <tr>
            <td><strong>Claude Code</strong></td>
            <td align="right"><strong>261.0 MB</strong></td>
            <td align="right"><strong>8.0× more RAM</strong></td>
          </tr>
        </tbody>
      </table>
    </td>
    <td width="24"></td>
    <td valign="top" align="center" width="50%">
      <strong>20 active sessions</strong>
      <table>
        <thead>
          <tr>
            <th>Tool</th>
            <th>PSS</th>
            <th>Comparison</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td><strong>Kraivcode</strong></td>
            <td align="right"><strong>90.6 MB</strong></td>
            <td align="right">baseline</td>
          </tr>
          <tr>
            <td><strong>Claude Code</strong></td>
            <td align="right"><strong>3376.8 MB</strong></td>
            <td align="right"><strong>37.3× more RAM</strong></td>
          </tr>
        </tbody>
      </table>
    </td>
  </tr>
</table>

</div>

### Headless sessions (swarm workers)

Swarm workers run headless, so this is the number that matters when you fan out
agents. Each session completed 5 real model turns (file listing, file read, repo
search, summary, reply) with tool calls, then total PSS of every process was
measured.

<div align="center">

| Concurrent headless sessions | Kraivcode | Claude Code | Comparison |
|---:|---:|---:|---:|
| 1 | **32.6 MB** | 261.0 MB | **8.0× less RAM** |
| 5 | **51.0 MB** | 908.6 MB | **17.8× less RAM** |
| 10 | **66.7 MB** | 1749.7 MB | **26.2× less RAM** |
| 20 | **90.6 MB** | 3376.8 MB | **37.3× less RAM** |
| Each additional session | **~3.1 MB** | ~164 MB | **~54× less RAM** |

</div>

### Time to first frame

<div align="center">

| Tool | Time to first frame | Range | Comparison |
|---|---:|---:|---:|
| **Kraivcode** | **14.0 ms** | 10.1–19.3 ms | baseline |
| Codex CLI | 882.8 ms | 742.3–1640.9 ms | **63.1× slower** |
| OpenCode | 1035.9 ms | 922.5–1104.4 ms | **74.0× slower** |
| GitHub Copilot CLI | 1518.6 ms | 1357.4–1826.8 ms | **108.5× slower** |
| Cursor Agent | 1949.7 ms | 1711.0–2104.8 ms | **139.3× slower** |
| Claude Code | 3436.9 ms | 2032.7–8927.2 ms | **245.5× slower** |

</div>

### Time to first input

<div align="center">

| Tool | Time to first input | Range | Comparison |
|---|---:|---:|---:|
| **Kraivcode** | **48.7 ms** | 30.3–62.7 ms | baseline |
| Codex CLI | 905.8 ms | 760.1–1675.7 ms | **18.6× slower** |
| OpenCode | 1047.9 ms | 931.1–1116.9 ms | **21.5× slower** |
| GitHub Copilot CLI | 1583.4 ms | 1422.8–1880.0 ms | **32.5× slower** |
| Cursor Agent | 1978.7 ms | 1727.3–2130.0 ms | **40.6× slower** |
| Claude Code | 3512.8 ms | 2137.4–9002.0 ms | **72.2× slower** |

</div>

> **Provenance.** Measured 2026-09-29 on the engine Kraivcode is built on
> (upstream v0.89.19-dev), not re-run against this fork. Read them as the
> shape of the inherited architecture. Reproduce with
> `python3 scripts/bench_headless_memory.py` and
> `python3 scripts/bench_startup.py`; for a Kraivcode-specific figure, run
> those scripts in this repo and publish the result.

---

<div align="left">

## Memory (Agent memory)

</div>

Every turn is embedded as a semantic vector. Each new turn queries a graph of
memories and pulls in the entries that are close enough to matter, so relevant
context arrives without the agent spending tokens on a memory tool call.

Two paths for that. The embedding hits can be injected directly into the
conversation, or handed to a memory sideagent that verifies they are actually
relevant and does additional retrieval work before injecting anything. The
result is a memory system that recalls on its own without becoming a token
burner.

Memories also have to be extracted, not just retrieved. Extraction runs on a
trigger — semantic drift, N turns since the last pass, session end — and a
memory sideagent writes the results into the graph.

<div align="left">

| Surface | Purpose |
|---|---|
| Automatic recall | Embedding hits injected into the turn |
| Verified recall | A memory sideagent checks relevance first |
| Explicit tools | The agent searches or stores memory deliberately |
| Session search | Traditional RAG across previous sessions |
| Ambient consolidation | Reorganises the graph, checks staleness, resolves conflicts |

</div>

---

<div align="left">

## Swarm

</div>

Spawn two or more agents in the same repository and the server manages them
natively. When agent A edits a file agent B has already read, B is notified. B
can ignore it, or open the diff and reconcile. Agents can DM one agent,
broadcast to the server, or target only agents in the same repo — so you can
spawn several sessions over one codebase and have conflicts handled rather than
discovered.

Agents can also spawn swarms on their own, turning themselves into a coordinator
with workers. Teams, channels, and completion status are managed for you, headed
or headless.

Root reasoning stays separate from worker effort:

```toml
[agents]
swarm_root_effort = "low"        # /effort swarm
swarm_deep_root_effort = "high"  # /effort swarm-deep
```

Both default to `max`. Accepted levels are `none`, `minimal`, `low`, `medium`,
`high`, `xhigh`, `max`, mapped onto the provider's supported range. Worker
`swarm_effort` is unaffected. Overrides are `JCODE_SWARM_ROOT_EFFORT` and
`JCODE_SWARM_DEEP_ROOT_EFFORT`.

---

<div align="left">

## Browser Automation

</div>

A built-in `browser` tool gives the agent real browser control inside a session.
The wired backend today is Firefox via Firefox Agent Bridge.

```bash
kraivcode browser status
kraivcode browser setup
```

Current built-in actions: `status`, `setup`, `open`, `snapshot`, `get_content`,
`interactables`, `click`, `type`, `fill_form`, `select`, `wait`, `screenshot`,
`eval`, `scroll`, `upload`, `press`.

The UI summarises browser calls compactly — a URL opened, a selector clicked, a
field filled — without echoing sensitive typed text. The provider architecture
is in place for other backends, including Chrome-style remote debugging.

---

<div align="left">

## OAuth and Providers

</div>

Kraivcode speaks to subscription-backed OAuth flows and plain API providers
alike, so you can use the models you already pay for and fall back to direct API
keys when a quota runs out.

<div align="left">

| | |
|---|---|
| Login providers | **57** |
| OpenAI-compatible profiles | **45** |
| Multi-account | Built in — `/account` swaps to your second subscription |

</div>

```bash
kraivcode login --provider claude
kraivcode login --provider openai
kraivcode login --provider gemini
kraivcode login --provider copilot
kraivcode login --provider azure
kraivcode login --provider bedrock
kraivcode login --provider openrouter
kraivcode login --provider groq
kraivcode login --provider deepseek
kraivcode login --provider lmstudio
kraivcode login --provider ollama
kraivcode login --provider openai-compatible
```

Built-in OpenAI-compatible profile ids include `openrouter`, `orcarouter`,
`deepseek`, `zai`, `kimi`, `moonshotai`, `opencode`, `opencode-go`, `302ai`,
`baseten`, `conifer`, `cortecs`, `comtegra`, `fpt`, `firmware`, `huggingface`,
`nebius`, `scaleway`, `stackit`, `groq`, `mistral`, `perplexity`, `togetherai`,
`deepinfra`, `fireworks`, `novita`, `minimax`, `xai`, `chutes`, `cerebras`,
`belvedir`, `nvidia-nim`, `xiaomi-mimo`, `meta-muse`, `celeris`, `yolo-auto`,
`omniroute`, `agentrouter`, `alibaba-coding-plan`, `lmstudio`, `ollama`,
`anthropic-api`, `openai-api`, `gemini-api`, and `openai-compatible`. Each
profile sets only the endpoint and key variable; you still pick the model with
`/model`.

### Scripted and remote setup

For agents, CI, and SSH sessions:

```bash
# Secret-safe setup for a hosted OpenAI-compatible API.
printf '%s' "$MY_API_KEY" | kraivcode provider add my-api \
  --base-url https://llm.example.com/v1 \
  --model my-model-id \
  --api-key-stdin \
  --set-default \
  --json

# Smoke test the profile.
kraivcode --provider-profile my-api auth-test --prompt 'Reply exactly OK'

# Use it directly.
kraivcode --provider-profile my-api run 'hello'
```

Local runtimes that need no key:

```bash
kraivcode provider add local-vllm \
  --base-url http://localhost:8000/v1 \
  --model Qwen/Qwen3-Coder-30B-A3B-Instruct \
  --no-api-key \
  --set-default
```

Useful flags: `--api-key-env NAME` to reference an existing variable,
`--api-key-stdin` to avoid shell history, `--context-window TOKENS`,
`--model-catalog` to merge the endpoint's `/models` response, and `--overwrite`
to replace an existing profile.

The generated profile is plain TOML and safe to edit by hand:

```toml
[provider]
default_provider = "my-api"
default_model = "my-model-id"

[providers.my-api]
type = "openai-compatible"
base_url = "https://llm.example.com/v1"
api_key_env = "JCODE_PROVIDER_MY_API_API_KEY"
default_model = "my-model-id"
# Stop model names like `gpt-5-*` from auto-enabling reasoning_effort on
# gateways that reject it.
disable_reasoning_heuristics = true

[[providers.my-api.models]]
id = "my-model-id"
context_window = 128000
reasoning = true
reasoning_effort = "high"
```

Anthropic Messages-compatible gateways use the same surface with
`type = "anthropic-compatible"`, and can select bearer, custom-header, or no
authentication, plus per-profile headers.

### Backends that need non-standard fields

Some OpenAI-compatible servers only enable thinking when the request body
carries specific top-level fields — NVIDIA NIM DeepSeek-V4 models, for example,
need `chat_template_kwargs` or they reply without reasoning. Inject them per
profile:

```toml
[providers.my-nim.extra_body.chat_template_kwargs]
thinking = true
reasoning_effort = "high"
```

Or globally via `JCODE_OPENAI_EXTRA_BODY='{"chat_template_kwargs":{...}}'`, which
can live in the provider's env file next to the API key. Injected keys merge
last and win on collision; invalid values are logged and ignored rather than
failing the request.

### Streaming timeouts

`JCODE_STREAM_IDLE_TIMEOUT_SECS` raises the base idle timeout (default 180s) for
models that think silently before emitting tokens. High reasoning efforts scale
it automatically. Also settable as `[provider] stream_idle_timeout_secs`.

### Headless and SSH logins

```bash
# Print a resumable auth URL, complete it later
kraivcode login --provider openai --print-auth-url --json
kraivcode login --provider openai --callback-url 'http://localhost:1455/auth/callback?...'

# Gemini: paste the auth code instead
kraivcode login --provider gemini --auth-code '...'

# Copilot device flow
kraivcode login --provider copilot --print-auth-url --json
kraivcode login --provider copilot --complete
```

`--no-browser` (alias `--headless`) prints the URL or QR and waits for a manual
paste instead of trying to open a browser. Pending login state lives under
`~/.jcode/pending-login/`, expires automatically, and is cleaned up when a new
scripted login starts or resumes.

### MCP config files

MCP config is separate from `config.toml`:

- `~/.jcode/mcp.json` — global servers
- `.jcode/mcp.json` — project-local servers

Claude Code compatibility is read live on every load, so edits take effect
without a stale snapshot: `~/.claude.json` (top-level `mcpServers` plus
per-project servers under `projects.<abs_path>.mcpServers`), `.mcp.json` at the
repo root, and `.claude/mcp.json` as a legacy fallback. A one-time import from
`~/.codex/config.toml` runs when `~/.jcode/mcp.json` does not yet exist; that
file becomes Kraivcode-owned afterward, and imported environment values may
contain secrets.

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "/path/to/mcp-server",
      "args": ["--root", "/workspace"],
      "env": {},
      "shared": true
    },
    "websearch": {
      "command": "/path/to/slow-mcp-server",
      "timeout_secs": 120
    }
  }
}
```

Requests to an MCP server (`tools/call`, `tools/list`, `initialize`) time out
after 30 seconds by default; raise it with `timeout_secs` on that server. Both
the canonical `mcpServers` key and the historical `servers` key are accepted.
Stdio servers are supported; HTTP and SSE entries are recognised and skipped
with a log line.

---

<div align="left">

## Customizability and Self-Dev

</div>

Kraivcode can modify its own source. Enter self-dev mode and the agent edits,
builds, and tests the checkout, then reloads its own binary and keeps working —
in as many sessions as you like.

Use a frontier model for this. This is a large Rust codebase, and weaker models
make subtle breaking changes here.

<div align="left">

| Command | Effect |
|---|---|
| `/selfdev` | Spawn a self-dev session in a separate terminal |
| `/rebuild` | `git pull --ff-only` + release build in the background, auto-reload |
| `/reload` | Swap in a newer binary without rebuilding |
| `/restart` | Restart on the current binary, session preserved |

</div>

---

<div align="left">

## Misc.

</div>

The devil is in the details. Some of the smaller things that add up:

**Cache awareness.** Anthropic's Claude cache goes cold after 5 minutes. Kraivcode
warns you when the cache went cold and notifies you on an unexpected cache miss.

**Interleaved input.** Typed input is by default sent as soon as it can be, without
breaking the KV cache. Submit with <kbd>Shift</kbd>+<kbd>Enter</kbd> instead and it
queues, waiting for the agent to fully finish its turn.

**Cross-harness resume.** Claude Code broke on you? Resume that session from
Kraivcode and continue where you left off. Session resume is supported for Codex,
Claude Code, OpenCode, and pi.

**Skill injection.** Skills are not all loaded at startup. The conversation is
embedded as a semantic vector, and a skill is injected automatically on an
embedding hit — the same mechanism as memory. The agent also has a skill tool for
manual activation, and slash commands work too.

**Agent grep.** Adds file structure information (functions, their displacement,
and so on) to grep returns, so the agent can infer more about a file without
reading it. A harness-level integration adaptively truncates returns based on what
the agent has already seen, which saves a lot of context.

**Browser bridge setup.** Ask your agent to set up Firefox Agent Bridge and you
have browser automation available in Kraivcode too.

---

<div align="left">

## Quick Start

</div>

```bash
# Launch the TUI
kraivcode

# Run a single command non-interactively
kraivcode run "say hello"

# Resume a previous session by memorable name
kraivcode --resume fox

# Run as a persistent background server, then attach more clients
kraivcode serve
kraivcode connect

# Send voice input from your configured STT command
kraivcode dictate
```

Then, inside the TUI:

```text
/plan add a healthcheck endpoint    read-only research, then a plan card
Tab                                  cycle personas
/skills                              searchable skill browser
/resume                              session picker with history preview
/account                             swap provider accounts
/keys                                scan for terminal keybinding conflicts
```

---

<div align="left">

## Command Surface

</div>

<div align="left">

| Area | Commands |
|---|---|
| Personas and planning | `/plan` `/review` `/judge` `/autoreview` `/autojudge` |
| Parallel work | `/swarm` `/effort` `/fork` `/split` `/transfer` `/catchup` `/back` |
| Loop modes | `/improve` `/refactor` `/overnight` `/goals` `/poke` |
| Interface | `/alignment` `/skills` `/skills-text` `/context` `/info` `/usage` `/compact-notifications` |
| Providers | `/login` `/account` `/model` `/hosted` `/subscription` |
| Build | `/selfdev` `/rebuild` `/reload` `/restart` `/update` `/changelog` |
| Session | `/resume` `/sessions` `/memory` `/fast` `/subagent-model` |
| Utilities | `/config` `/diff` `/keys` `/test` `/rewind` `/clear` `/fix` |
| Voice | `/voice` `/dictate` |

</div>

Type `/` to browse the registered commands with their descriptions, and
`/help <command>` to expand any one of them in full.

---

<div align="left">

## Configuration

</div>

```toml
# ~/.jcode/config.toml

[display]
emoji = false                 # swap emoji for ASCII markers
alignment = "left"            # or "center"

[provider]
default_provider = "my-api"
default_model = "my-model-id"
stream_idle_timeout_secs = 180

[keybindings]
voice_input = "ctrl+space"

[dictation]
command = "~/.local/bin/my-whisper-script --grammar-target code"
mode = "append"               # insert | append | replace | send
timeout_secs = 60

[features]
update_channel = "stable"
```

`/config` prints the active configuration, `/config init` writes a default file,
and `/config edit` opens it in `$EDITOR`.

---

<div align="left">

## Further Reading

</div>

Architecture:

- [Memory architecture](docs/MEMORY_ARCHITECTURE.md)
- [Swarm architecture](docs/SWARM_ARCHITECTURE.md)
- [Swarm task graph](docs/SWARM_TASK_GRAPH.md)
- [Server architecture](docs/SERVER_ARCHITECTURE.md)
- [Multi-session client architecture](docs/MULTI_SESSION_CLIENT_ARCHITECTURE.md)
- [Safety system](docs/SAFETY_SYSTEM.md)
- [Ambient mode](docs/AMBIENT_MODE.md)
- [Desktop panels](docs/PANELS.md)
- [Browser provider protocol](docs/BROWSER_PROVIDER_PROTOCOL.md)
- [OpenAI WebSocket transport](docs/OPENAI_WEBSOCKET.md)

Interface and input:

- [TUI colour configuration](docs/TUI_COLOR_CONFIGURATION.md)
- [Terminal capabilities](docs/TERMINAL_CAPABILITIES.md)
- [Shift+Enter behaviour](docs/SHIFT_ENTER.md)
- [Keymap conflicts](docs/KEYMAP_CONFLICTS.md)
- [Message and voice input](docs/MESSAGE_VOICE.md)
- [Hooks](docs/HOOKS.md)
- [Spawn hook](docs/SPAWN_HOOK.md)

Operations:

- [Windows notes](docs/WINDOWS.md)
- [Wrappers and shell integration](docs/WRAPPERS.md)
- [Native SSH](docs/NATIVE_SSH.md)
- [Remote compile](docs/REMOTE_COMPILE.md)
- [Soft interrupt](docs/SOFT_INTERRUPT.md)
- [Resume behaviour](docs/RESUME_BEHAVIOR.md)
- [Provider doctor](docs/PROVIDER_DOCTOR.md)
- [Auth credential sources](docs/AUTH_CREDENTIAL_SOURCES.md)
- [AWS Bedrock provider](docs/AWS_BEDROCK_PROVIDER.md)
- [Telemetry](TELEMETRY.md)
- [OAuth notes](OAUTH.md)

---

<div align="left">

## Detailed Installation

</div>

### Setup

If you want another agent to set up Kraivcode for you, give it this prompt:

```text
Set up Kraivcode on this machine for me.

1. Detect the operating system, available package managers, and shell environment,
   then install Kraivcode from source:

     git clone https://github.com/nkswalih/kraivcode.git
     cd kraivcode
     cargo build --release --bin kraivcode
     install -m 755 target/release/kraivcode ~/.local/bin/jcode

   On Windows, copy target\release\kraivcode.exe to a directory on PATH instead.

2. Verify that `kraivcode` is on my PATH.
3. Launch `kraivcode` once in a new terminal to confirm it starts.
4. Before any interactive login flow, assess which providers are already available
   non-interactively. Check existing local credentials, config files, CLI sessions,
   and environment variables such as:
   - Claude: ~/.jcode/auth.json, ~/.claude/.credentials.json, ANTHROPIC_API_KEY
   - OpenAI: ~/.jcode/openai-auth.json, ~/.codex/auth.json, OPENAI_API_KEY
   - Gemini: ~/.jcode/gemini_oauth.json, ~/.gemini/oauth_creds.json
   - GitHub Copilot: existing auth under ~/.config/github-copilot/
   - Azure OpenAI: ~/.config/jcode/azure-openai.env, AZURE_OPENAI_*, or `az login`
   - OpenRouter: OPENROUTER_API_KEY
5. Prefer whichever provider is already configured, and verify it with
   `kraivcode auth-test --all-configured`.
6. Only if no usable provider is already configured, guide me through the minimal
   manual step:
   - kraivcode login --provider claude
   - kraivcode login --provider openai
   - kraivcode login --provider gemini
   - kraivcode login --provider copilot
7. Run a smoke test with `kraivcode run "say hello"` and confirm it works.
8. If I want browser automation, run `kraivcode browser status`, then
   `kraivcode browser setup` if it is not ready.
9. Explain any manual step that still needs me, especially browser OAuth, device
   login, API key entry, or browser extension approval.
```

### Build from source

```bash
git clone https://github.com/nkswalih/kraivcode.git
cd kraivcode
cargo build --release
```

For local self-dev and refactor work, prefer the wrapper — it uses `sccache`
when available and picks a working local linker (`clang + lld`) instead of
assuming your `mold` configuration is valid:

```bash
scripts/dev_cargo.sh build --release -p kraivcode --bin kraivcode
scripts/dev_cargo.sh --print-setup
```

### Uninstall

Remove the binary and any launcher you created. Your config, auth, and sessions
live under `~/.jcode/` and are left alone:

```bash
# macOS & Linux
rm -f ~/.local/bin/jcode
```

```powershell
# Windows
Remove-Item "$env:LOCALAPPDATA\kraivcode\bin\jcode.exe"
```

---

<div align="left">

## Platform Support

</div>

<div align="left">

| Platform | Status |
|---|---|
| **Linux** x86_64 / aarch64 | Fully supported |
| **macOS** Apple Silicon & Intel | Supported |
| **Windows** x86_64 | Supported (native + WSL2) |
| **Termux** aarch64 / x86_64 | Supported with `pkg install glibc patchelf` |

</div>

---

<div align="left">

## Lineage

Kraivcode is built on **[jcode](https://github.com/1jehuang/jcode)**,<br>
which is MIT licensed. The upstream copyright notice is preserved in [LICENSE](LICENSE),<br>
as the MIT terms require.

Personas, `ask_user`, the Plan tool allowlist, the skills browser, the prompt gutter,<br>
the floating intent panel, and the paste classifier are this fork's work.

<br>

[![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)

</div>
