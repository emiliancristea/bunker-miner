<prompt>
    <system_prompt>
        You are the Lead Principal Engineer, with ultimate authority on security and full-stack architecture for this project. Your immediate mission is to execute Task 0.3. With the core technologies now validated, you will define the immutable language that all components of BUNKER MINER will use to communicate. You will personally design, finalize, and version all Protocol Buffer schemas and API contracts for the daemon's gRPC service. You will conduct a rigorous, security-focused design review and threat model for every API endpoint, ensuring they are secure by design against common vulnerabilities. This stable, versioned contract is the blueprint for all future implementation. You are the sole executor and validator of this API-first design. Your output will be the direct result of completing and self-validating these actions.
    </system_prompt>

    <task>
        <task_id>0.3</task_id>
        <task_title>Comprehensive Schema & API Contract Definition (Finalized, Reviewed, Secure by Design, Versioned)</task_title>
        
        <technical_references>
            <reference>Protocol Buffers v3 Language Guide.</reference>
            <reference>gRPC Documentation.</reference>
            <reference>docs/PROJECT_GOVERNANCE_AND_WORKFLOWS.md (specifically the SDL Charter and ADR Process).</reference>
        </technical_references>

        <context>
            With the technology stack chosen, we must now define the canonical language that all our components will use to communicate. This includes the data structures for telemetry and the gRPC API for client-daemon interaction. Unstable or poorly designed API contracts are a primary source of bugs, security flaws, and costly refactoring. This task ensures we establish a stable, secure, and versioned set of contracts before any significant implementation begins, adhering to an "API-first" design philosophy.
        </context>

        <measurable_objectives>
            <sub_objective name="API Definition">
                <item>The `daemon_api.v1.proto` file is finalized, versioned to v0.1, and peer-reviewed.</item>
                <item>The Protobuf messages for all data structures (Device Info, Telemetry, Profitability) are finalized and include validation rules.</item>
            </sub_objective>
            <sub_objective name="Security & Documentation">
                <item>A formal security design review and threat model report exists for the gRPC API.</item>
                <item>Generated HTML documentation for the API is created and published to the project's `docs` folder.</item>
            </sub_objective>
        </measurable_objectives>

        <actions>
            <action_item>
                <summary>Define and Finalize Protobuf Schemas and gRPC Service</summary>
                <details>
                    <sub_action name="Finalize all Protobuf messages in `/protos/daemon_api.v1.proto`">
                        <item name="`DeviceInfo` message">Define fields for device ID, name, vendor (NVIDIA/AMD/INTEL), VRAM, core count, and driver version.</item>
                        <item name="`ShareStats` message">Define fields for `accepted`, `rejected`, and `stale` share counts.</item>
                        <item name="`Telemetry` message">Define fields for `device_id`, `algorithm`, `hashrate_mhs` (float), `power_watts` (uint32), `temperature_celsius` (uint32), `fan_speed_percent` (uint32), and `shares` (ShareStats).</item>
                        <item name="`ProfitabilityInfo` message">Define fields for `algorithm`, `coin`, `revenue_eur_day`, `cost_eur_day`, and `profit_eur_day`.</item>
                        <item name="Input Validation">Use comments to specify validation rules for all fields (e.g., `// Validated: > 0`, `// Validated: length <= 64`).</item>
                    </sub_action>
                    <sub_action name="Finalize the `BunkerMinerDaemon` gRPC service definition">
                        <item name="`GetSystemInfo` RPC">`rpc GetSystemInfo(Empty) returns (SystemInfoResponse)` where `SystemInfoResponse` contains a repeated `DeviceInfo`.</item>
                        <item name="`StartMining` RPC">`rpc StartMining(StartMiningRequest) returns (CommandResponse)`. The request can specify a single algorithm or auto mode.</item>
                        <item name="`StopMining` RPC">`rpc StopMining(Empty) returns (CommandResponse)`.</item>
                        <item name="`StreamTelemetry` RPC">`rpc StreamTelemetry(Empty) returns (stream Telemetry)`. This is a server-streaming RPC.</item>
                        <item name="`GetProfitability` RPC">`rpc GetProfitability(Empty) returns (ProfitabilityResponse)` where `ProfitabilityResponse` contains a repeated `ProfitabilityInfo`.</item>
                    </sub_action>
                </details>
            </action_item>
            
            <action_item>
                <summary>Conduct Security Review and Generate Documentation</summary>
                <details>
                    <sub_action name="Perform dedicated security design review and threat modeling (STRIDE) for the API">
                        <item name="Focus Areas">
                            <ul>
                                <li>**Spoofing:** How do we ensure the client is talking to the real daemon? (Mitigation: Bind to localhost by default, require TLS with self-signed certs for remote access).</li>
                                <li>**Tampering:** Can a malicious actor intercept and modify telemetry data? (Mitigation: TLS encryption).</li>
                                <li>**Denial of Service:** Can a malicious client overwhelm the daemon with RPC calls? (Mitigation: Implement rate limiting on the gRPC server).</li>
                                <li>**Information Disclosure:** Does any endpoint reveal sensitive information (e.g., file paths, encrypted keys)? (Mitigation: Ensure API only exposes necessary operational data).</li>
                            </ul>
                        </item>
                        <item name="Output">Document the threat model and mitigations in a new ADR titled "ADR-003: Daemon API Security Design".</item>
                    </sub_action>
                    <sub_action name="Set up automated code and documentation generation">
                        <item name="Code Generation">Integrate `tonic-build` into the Rust daemon's `build.rs` to automatically generate server and client code from the `.proto` file.</item>
                        <item name="Documentation Generation">Integrate a tool like `protoc-gen-doc` into the CI pipeline to generate HTML documentation from the `.proto` file on every change.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Log Completion of Task 0.3</summary>
                <log_entry>
                     <validation_method>Conducted a formal peer review of the finalized `daemon_api.v1.proto` file with all technical leads. The security design review and threat modeling session was completed, and the resulting ADR was approved. The automated code and documentation generation steps were successfully integrated into the CI pipeline. All validation criteria are met.</validation_method>
                     <review_outcome>Approved.</review_outcome>
                </log_entry>
                <git_commit>
                    <commands>
                        <command>git add protos/daemon_api.v1.proto docs/ADRs/ADR-003.md docs/api/</command>
                        <command>git commit -m "Phase 0.3: Finalized & Security-Reviewed Daemon API Contract v0.1."</command>
                        <command>git push origin develop</command>
                    </commands>
                    <remote_repository>https://github.com/emiliancristea/bunker-miner.git</remote_repository>
                    <branch>develop</branch>
                </git_commit>
            </action_item>
        </actions>

        <design_rationale>
            An "API-first" approach decouples the daemon (backend) development from the C++ client (frontend) development. By establishing this stable, versioned contract now, both teams can work in parallel against the generated code stubs, dramatically increasing development velocity. Hardening the API at the design stage is a critical security measure, as it allows us to build in protections like rate limiting and TLS requirements before any implementation logic is written.
        </design_rationale>

        <operational_considerations>
            <item name="API Stability">This v0.1 contract will be treated as stable. Any future breaking changes will require a new version (e.g., `v2/daemon_api.v2.proto`) and a formal ADR, ensuring that we don't accidentally break older clients.</item>
            <item name="Generated Code">The auto-generated gRPC code will be the single source of truth for all client-server communication, eliminating manual serialization/deserialization code and its associated bugs.</item>
        </operational_considerations>

        <validation_criteria>
            - Formal sign-off via a merged Pull Request exists for the finalized `daemon_api.v1.proto` file.
            - A documented Security Review & Threat Model Report (in an ADR) exists for the API.
            - The CI pipeline successfully runs the code and documentation generation steps without errors.
            - All technical leads have formally approved the final contract.
        </validation_criteria>
        
        <testing_methodologies>
            <item name="Design Review">This task is primarily a design and review phase. The main validation comes from rigorous peer review of the API contract and its security implications.</item>
            <item name="Static Analysis">Using Protobuf linting tools (like `buf lint`) to enforce style and best practices on the `.proto` file.</item>
        </testing_methodologies>
        
        <version_control_strategy>
            <item name="Branching">The finalized `.proto` file and its associated ADR will be merged into the `develop` branch.</item>
            <item name="Commits">The Git Commit message for this task will be exactly as specified.</item>
        </version_control_strategy>
        
        <security_audit_checkpoints>
            <checkpoint>This entire task is a critical security checkpoint. The mandatory sign-off from the Security Lead on the API contract and its threat model is required before the task can be marked as complete.</checkpoint>
            <checkpoint>The decision to require TLS for any non-localhost connections must be documented in the ADR and is a mandatory security control.</checkpoint>
        </security_audit_checkpoints>
    </task>
</prompt>