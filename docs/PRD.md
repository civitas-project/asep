
<!-- @req:FR-LEADER-01:MUST OpenCode project presets, generated presets, and the global configuration on this machine shall use leader as the primary role name. The global default shall select leader, with unrelated settings preserved. -->
<!-- @req:FR-LEADER-02:MUST The Rust role, message schema, and MCP tools shall use Role::Leader and the wire value leader. The former wire value lead shall be rejected without alias or message migration. -->
<!-- @req:FR-ACCEPT-01:MUST A trusted local runner shall be able to mark an exact snapshot ACCEPTED without a remote service only after all mandatory gates pass and locally protected evidence is valid for that snapshot and policy. Missing, stale, untrusted, or failed evidence shall prevent acceptance. -->

`FR-ACCEPT-02` (GitHub Actions merge authorization) is deferred, not fulfilled. P0 adds no workflow, GitHub App, or ruleset. Existing branch protections still apply; local `ACCEPTED` does not authorize a merge. Codeberg remains a mirror.

Builds and verification continue to use `nightly-2026-11-14`. Executor `DONE` and judge `PASS` do not mean `ACCEPTED`.
