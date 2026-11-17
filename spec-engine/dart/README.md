# ASEP Dart Runner

Universal zero-dependency Dart runner for the **Axiomatic Systems Engineering Protocol (ASEP)** native Rust engine.

## Usage

Run directly via the Dart SDK:

```bash
# Execute traceability and SDLC gate check
dart run ./bin/run.dart check .

# Pack agent skills into local workspace
dart run ./bin/run.dart pack --local --agent antigravity -y

# Start stdio Model Context Protocol (MCP) server
dart run ./bin/run.dart mcp
```

