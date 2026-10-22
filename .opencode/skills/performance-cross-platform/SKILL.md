---
name: performance-cross-platform
description: Keep Gaggle responsive, resource-bounded, and reliable across supported macOS and Linux targets.
version: 1.0.0
---

# Performance and Cross-platform Skill

## Mission
Deliver consistent behavior and reasonable resource use without sacrificing correctness or security.

## Performance principles
- Measure before optimizing.
- Avoid blocking async executor threads.
- Stream large outputs and apply backpressure.
- Bound model context, tool output, queues, concurrent processes, and retained events.
- Avoid repeated full-project scans when incremental indexes or file watchers are suitable.
- Debounce UI updates without delaying important approval or failure events.
- Keep long-running execution independent from UI component lifecycles.
- Profile memory and CPU during long chats, large repositories, and concurrent tasks.

## Platform checklist
Verify:
- Apple Silicon and Intel support if both are claimed.
- Supported Linux architectures and distributions.
- Credential store availability and fallback behavior.
- Shell discovery and controlled environment handling.
- PTY resize, process-tree termination, and signal behavior.
- Symlinks, permissions, Unicode paths, and large files.
- File watcher limits and missed-event recovery.
- WebView/runtime prerequisites.
- Install, update, uninstall, and diagnostic flows.

## Rules
- Do not assume a successful compile proves runtime compatibility.
- Do not claim support for an untested platform.
- Make unavailable platform capabilities visible to the user.
- Record benchmarks with workload, machine, build mode, and measurement method.
