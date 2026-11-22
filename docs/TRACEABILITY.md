
@rel:satisfies:FR-LEADER-01->FR-LEADER-01
@rel:verifies:FR-LEADER-01->FR-LEADER-01 "New preset installs leader as primary and does not generate a lead key"
@rel:satisfies:FR-LEADER-02->FR-LEADER-02
@rel:verifies:FR-LEADER-02->FR-LEADER-02 "MCP rejects the lead role; protocol test also checks leader v2 serialization and v1 envelope rejection"

### P0 evidence and deployment limits

<!-- @test:FR-LEADER-01 -->
`targets_test` passes 13/13. New preset uses `leader` as primary and does not create the `lead` key. Global config verified choosing `leader`; model and permissions unchanged.

<!-- @test:FR-LEADER-02 -->
`agent_protocol_test` (14/14), `mcp_test` (12/12), and `integration_check_test` (14/14) pass. Tests cover `leader` v2 serialization, rejection of old roles and old versions, and ACK rules.

<!-- @test:FR-ACCEPT-01 -->
`cargo test -p asep --offline --locked --test acceptance_test test_accept_offline_protected_fixture -- --ignored --exact --nocapture` passes. Valid Git snapshot produces evidence with commit identity, snapshot digest and policy, and seven mandatory gates with exit 0 and no skip. Bad digest rejected before the gate; snapshot with valid SHA and digest but failing test rejected at gate `Tests` (exit 101). Neither rejection writes acceptance evidence. The test uses isolated Linux namespaces without `sudo` or `/etc` host changes. Separate probe confirms read-only snapshot, evidence storage not mounted in the candidate sandbox, and only loopback interface available. This evidence applies to the fixture, not the deployment runner host.

@rel:satisfies:FR-ACCEPT-01->FR-ACCEPT-01
@rel:verifies:FR-ACCEPT-01->FR-ACCEPT-01 "Provisioned offline fixture proves the success path and rejections without acceptance evidence"

- Merge: `FR-ACCEPT-02` deferred outside P0. Draft GitHub Actions removed before deployment. Existing branch protection unchanged; local `ACCEPTED` is not merge permission.
