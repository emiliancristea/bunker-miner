<prompt>
    <system_prompt>
        You are the Lead Principal Engineer, with ultimate authority on security and full-stack architecture for this project. Your immediate mission is to execute Task 1.3. The headless daemon is now functional but is a black box. You will now give it a voice. You will personally implement a secure, stable, and well-defined gRPC API server within the daemon, based on the v0.1 contract you finalized in Phase 0. You will expose endpoints for system information, mining controls, and, most critically, a real-time telemetry stream. You will also build a simple command-line client to serve as a test harness and debugging tool for this new API. You are the sole executor and validator of this crucial communication layer. Your output will be the direct result of completing and self-validating these actions.
    </system_prompt>

    <task>
        <task_id>1.3</task_id>
        <task_title>Rust Daemon - gRPC API & Telemetry Service</task_title>
        
        <technical_references>
            <reference>Finalized `/protos/daemon_api.v1.proto` API Contract (from Task 0.3).</reference>
            <reference>`tonic` (Rust gRPC library) Documentation.</reference>
            <reference>ADR-003: Daemon API Security Design.</reference>
        </technical_references>

        <context>
            The headless daemon is now functional but provides no way for external tools (like our future GUI or third-party monitoring scripts) to interact with it or observe its state. To enable this, we must implement a secure, stable, and well-defined API. This task involves embedding a gRPC server into the daemon to expose its core functionality and real-time data streams to authorized clients.
        </context>

        <measurable_objectives>
            <sub_objective name="API Server">
                <item>A secure gRPC server is integrated into the daemon and runs on a local port by default.</item>
                <item>The server correctly implements all RPCs defined in the `daemon_api.v1.proto` contract.</item>
            </sub_objective>
            <sub_objective name="Functionality">
                <item>The API successfully exposes endpoints to get static device info, start/stop mining, and stream real-time telemetry.</item>
                <item>A `bunker-miner-cli` test tool is created and can successfully interact with all API endpoints.</item>
            </sub_objective>
        </measurable_objectives>

        <actions>
            <action_item>
                <summary>Integrate and Implement gRPC Server</summary>
                <details>
                    <sub_action name="Set up the `tonic` gRPC server in the daemon's main application loop">
                        <item name="Code Generation">Use the `tonic-build` crate in the daemon's `build.rs` to generate the Rust server and client code from the `daemon_api.v1.proto` file.</item>
                        <item name="Server Initialization">In `main.rs`, spawn the `tonic` server as a separate, long-running Tokio task.</item>
                        <item name="Security Configuration">
                            <ul>
                                <li>By default, the gRPC server must bind to `127.0.0.1` (localhost) to prevent unintentional remote access.</li>
                                <li>Add configuration options to allow binding to `0.0.0.0` for remote access, but require `tls_cert_path` and `tls_key_path` to be set in this mode. This enforces secure communication for any non-local connection.</li>
                            </ul>
                        </item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Implement gRPC Service Handlers</summary>
                <details>
                    <sub_action name="Wire the gRPC handlers to the daemon's core logic modules">
                        <item name="State Management">The gRPC service implementation will hold a shared, thread-safe state (`Arc<Mutex<DaemonState>>`) that allows it to communicate with the other modules.</item>
                        <item name="`GetSystemInfo` Handler">This handler will query the `hardware` module and return the list of detected `MiningDevice`s, translated into `DeviceInfo` Protobuf messages.</item>
                        <item name="`StartMining`/`StopMining` Handlers">These handlers will send commands to the `miners` process management module to start or stop the supervised miner process.</item>
                        <item name="`StreamTelemetry` Handler">This is the most complex handler. It will:
                            <ol>
                                <li>Create a new `tokio::sync::mpsc` channel for the subscriber.</li>
                                <li>Register the channel's `Sender` with a central `Broadcaster` service within the daemon.</li>
                                <li>The daemon's main telemetry parser (from Task 1.2) will send every new `Telemetry` struct to this `Broadcaster`.</li>
                                <li>The broadcaster will then forward the message to all registered subscribers.</li>
                                <li>The handler will listen on the `Receiver` end of the channel and stream the messages back to the gRPC client.</li>
                            </ol>
                        </item>
                    </sub_action>
                </details>
            </action_item>
            
            <action_item>
                <summary>Develop a CLI Test Harness</summary>
                <details>
                    <sub_action name="Create a new Rust binary project: `/tools/bunker-miner-cli`">
                        <item name="gRPC Client">This tool will use the same `tonic-build` generated client code to connect to the daemon's gRPC server.</item>
                        <item name="Commands">Implement `clap` subcommands that map directly to the gRPC RPCs:
                            <ul>
                                <li>`bunker-miner-cli info`: Calls `GetSystemInfo` and prints the results.</li>
                                <li>`bunker-miner-cli start`: Calls `StartMining`.</li>
                                <li>`bunker-miner-cli stop`: Calls `StopMining`.</li>
                                <li>`bunker-miner-cli watch`: Calls `StreamTelemetry` and prints the live data to the console as it arrives.</li>
                            </ul>
                        </item>
                        <item name="Purpose">This CLI serves as the primary integration testing tool for the API and will be invaluable for future debugging and scripting.</li>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Log Completion of Task 1.3</summary>
                <log_entry>
                     <validation_method>Successfully launched the daemon, which started the gRPC server. Used the newly created `bunker-miner-cli` tool to connect to the daemon. Executed all CLI commands: `info` correctly returned the list of hardware; `start` and `stop` correctly controlled the mining process; `watch` successfully connected to the streaming endpoint and printed a real-time feed of telemetry data from the active miner. All validation criteria are met.</validation_method>
                     <review_outcome>Approved.</review_outcome>
                </log_entry>
                <git_commit>
                    <commands>
                        <command>git add daemon/src/grpc.rs daemon/build.rs tools/bunker-miner-cli/</command>
                        <command>git commit -m "Phase 1.3: Implemented Daemon gRPC API & Telemetry Service."</command>
                        <command>git push origin develop</command>
                    </commands>
                    <remote_repository>https://github.com/emiliancristea/bunker-miner.git</remote_repository>
                    <branch>develop</branch>
                </git_commit>
            </action_item>
        </actions>

        <design_rationale>
            Using gRPC provides a high-performance, strongly-typed, and language-agnostic API contract. This is crucial for ensuring seamless and reliable communication between our Rust daemon and the C++ client. The server-streaming model for telemetry is highly efficient, pushing data to clients as it becomes available rather than forcing clients to poll continuously. A dedicated CLI test harness is a fundamental aspect of API-first design, allowing the API to be fully tested and validated independently of the main GUI client.
        </design_rationale>

        <operational_considerations>
            <item name="Port Conflict">The daemon must allow the default gRPC port to be configured to avoid conflicts with other software. This will be an option in `config.toml`.</item>
            <item name="Firewall">On first run, especially on Windows, users may see a firewall prompt to allow the daemon to listen on a local port. The onboarding documentation should prepare them for this.</item>
            <item name="Security">The default localhost-only binding is a critical security measure. The documentation must clearly explain the risks and the requirement for setting up TLS if a user chooses to allow remote access.</item>
        </operational_considerations>

        <validation_criteria>
            - The `bunker-miner-cli` test tool can successfully call all defined gRPC endpoints on the daemon.
            - The `StreamTelemetry` RPC successfully streams live, accurate data from a running miner.
            - The gRPC server correctly refuses non-TLS connections when configured for remote access.
            - The API implementation is peer-reviewed and signed off.
        </validation_criteria>
        
        <testing_methodologies>
            <item name="Integration Testing">The primary validation method is using the `bunker-miner-cli` to perform end-to-end tests of the entire API against a live, running daemon.</item>
            <item name="Unit Testing">Unit tests will be written for the logic within the gRPC handlers, mocking the daemon's internal state to test different scenarios (e.g., daemon is busy, no devices found).</item>
        </testing_methodologies>
        
        <version_control_strategy>
            <item name="Branching">The gRPC server and CLI test harness are developed on a `feature/daemon-grpc-api` branch.</item>
            <item name="Commits">The Git Commit message for this task will be exactly as specified.</item>
        </version_control_strategy>
        
        <security_audit_checkpoints>
            <checkpoint>The Security Lead must review and approve the implementation of the TLS requirement for remote connections. This is a non-negotiable security control.</checkpoint>
            <checkpoint>A threat model review of the API (from Task 0.3) must be revisited to ensure the implementation correctly mitigates all identified risks (DoS, Information Disclosure, etc.).</checkpoint>
            <checkpoint>The input validation logic in each gRPC handler must be reviewed to ensure it protects against malformed requests.</checkpoint>
        </security_audit_checkpoints>
    </task>
</prompt>