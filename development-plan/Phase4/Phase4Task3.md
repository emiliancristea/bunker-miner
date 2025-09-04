<prompt>
    <system_prompt>
        You are the Lead Principal Engineer, with ultimate authority on security and full-stack architecture for this project. Your immediate mission is to execute Task 4.3. The fleet management backend is now live, but the local software is unaware of it. You will now bridge this gap. You will personally implement the "agent" logic within the Rust daemon, enabling it to operate in "fleet mode" and connect securely to the remote Controller API. You will also enhance the C++/Qt client to serve as a user-friendly interface for managing the API keys needed to link their rigs to their account. Finally, you will implement the remote control functionality, allowing users to issue commands from the web dashboard that are securely relayed and executed by their remote daemons. You are the sole executor and validator of this critical remote-control integration. Your output will be the direct result of completing and self-validating these actions.
    </system_prompt>

    <task>
        <task_id>4.3</task_id>
        <task_title>Daemon & Client - Fleet Integration & Remote Control</task_title>
        
        <technical_references>
            <reference>Finalized Fleet Management WebSocket API documentation (from Task 4.2).</reference>
            <reference>`tokio-tungstenite` crate documentation for WebSocket clients.</reference>
            <reference>C++/Qt and gRPC documentation for client-backend interaction.</reference>
        </technical_references>

        <context>
            The fleet management backend is ready, but the local daemon and client are still operating in a standalone mode. This task involves updating the local software to integrate with the new remote controller. The daemon must learn how to act as a remote "agent," and the client must provide the user with the tools to manage the link between their rigs and their web account. This completes the loop, enabling true centralized monitoring and control.
        </context>

        <measurable_objectives>
            <sub_objective name="Daemon Integration">
                <item>The Rust daemon can be successfully configured to operate in "fleet mode," establishing a persistent, authenticated WebSocket connection to the remote Controller API.</item>
                <item>The daemon in fleet mode correctly streams its telemetry to the controller and can execute remote commands (start/stop).</item>
            </sub_objective>
            <sub_objective name="Client Integration">
                <item>The C++/Qt client UI is updated with a "Fleet Management" section that allows users to generate and manage their rig API keys.</item>
                <item>A user can successfully perform a remote control action (e.g., stop mining) from the web dashboard, and the target rig executes the command.</item>
            </sub_objective>
        </measurable_objectives>

        <actions>
            <action_item>
                <summary>Implement Fleet Agent Logic in Rust Daemon</summary>
                <details>
                    <sub_action name="Enhance the daemon's configuration and main loop">
                        <item name="Fleet Mode Configuration">Add a `[fleet_mode]` section to `config.toml` with parameters: `enable = true`, `controller_url = "wss://api.bunkerminer.com/fleet/ws"`, and `api_key = "..."`.</item>
                        <item name="Agent Logic">
                            <ul>
                                <li>Create a new `fleet_agent` module.</li>
                                <li>If fleet mode is enabled, this module will be responsible for initiating and maintaining a persistent WebSocket connection to the Controller API using `tokio-tungstenite`.</li>
                                <li>It must implement an auto-reconnect mechanism with exponential backoff in case the connection drops.</li>
                            </ul>
                        </item>
                        <item name="Daemon-Controller Communication">
                            <ul>
                                <li>**Authentication:** On connection, the agent must send an authentication message containing its `api_key`.</li>
                                <li>**Telemetry:** The agent will take the standardized `Telemetry` structs from the miner parser and forward them as JSON messages over the WebSocket to the controller.</li>
                                <li>**Remote Command Handling:** The agent will listen for incoming JSON messages from the controller's WebSocket. It will parse these commands (e.g., `{ "command": "STOP_MINING" }`) and trigger the same internal functions that the local gRPC API uses to control the miner process. This reuses existing, tested logic.</li>
                            </ul>
                        </item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Enhance C++/Qt Client for API Key Management</summary>
                <details>
                    <sub_action name="Add a 'Fleet Management' section to the Settings page">
                        <item name="UI Implementation">This new UI section will allow a user who is logged into their BUNKER MINER web account (via an embedded web view or token exchange) to manage their rig API keys.</item>
                        <item name="Backend Interaction">The UI will make authenticated gRPC/REST calls to the new Fleet Management backend to:
                            <ul>
                                <li>List existing API keys (showing only the prefix for security).</li>
                                <li>Generate a new API key (the full key is shown only once, immediately after creation).</li>
                                <li>Revoke an existing API key.</li>
                            </ul>
                        </item>
                        <item name="User Experience">This provides a seamless, in-app way for a user to get the API key they need to configure their other, headless rigs to connect to their account.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Implement Remote Controls on Web Dashboard</summary>
                <details>
                    <sub_action name="Add control buttons to the web frontend">
                        <item name="UI Elements">In the rig detail view of the web dashboard, add buttons for "Start Mining," "Stop Mining," and "Restart Miner."</item>
                        <item name="Command Logic">When a user clicks a button, the web frontend will send a command message over its own WebSocket to the backend. The backend will then find the corresponding rig's WebSocket connection and relay the command to the target daemon.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Log Completion of Task 4.3</summary>
                <log_entry>
                     <validation_method>Conducted a full end-to-end test. Used the C++/Qt client to log into a web account and generate a new API key. Configured a separate, headless daemon instance on a different machine with this key and fleet mode enabled. Verified the daemon appeared on the web dashboard and streamed live telemetry. From the web dashboard, successfully issued "Stop" and "Start" commands and observed the remote daemon executing them correctly. All validation criteria are met.</validation_method>
                     <review_outcome>Approved.</review_outcome>
                </log_entry>
                <git_commit>
                    <commands>
                        <command>git add daemon/src/fleet_agent.rs client/src/fleet_ui.rs</command>
                        <command>git commit -m "Phase 4.3: Integrated Daemon & Client with Fleet Management System."</command>
                        <command>git push origin develop</command>
                    </commands>
                    <remote_repository>https://github.com/emiliancristea/bunker-miner.git</remote_repository>
                    <branch>develop</branch>
                </git_commit>
            </action_item>
        </actions>

        <design_rationale>
            Reusing the same internal control functions for both the local gRPC API and the remote WebSocket API is a key design principle. It ensures that the daemon's core logic remains consistent and well-tested, regardless of where the command originates. This reduces code duplication and minimizes the attack surface. Providing API key management directly within the main desktop client creates a seamless user experience for users expanding from a single rig to a multi-rig fleet.
        </design_rationale>

        <operational_considerations>
            <item name="Security">The API key is a long-lived, powerful credential. The UI and documentation must instruct users to treat it like a password. The ability to revoke keys immediately from the web dashboard is a critical security feature.</item>
            <item name="Network Traffic">For large fleets, the volume of WebSocket telemetry traffic to the backend will be significant. The backend and database must be designed to handle this continuous, high-volume data stream.</item>
        </operational_considerations>

        <validation_criteria>
            - A daemon configured in "fleet mode" successfully connects to and is managed by the remote controller.
            - A user can successfully generate and revoke API keys from the C++/Qt client.
            - Remote start/stop commands sent from the web dashboard are successfully executed by the target daemon.
            - The web dashboard accurately reflects the real-time status of the remotely controlled rig.
        </validation_criteria>
        
        <testing_methodologies>
            <item name="End-to-End (E2E) System Testing">The primary validation is a full E2E test involving three components: the C++ client (for key management), the web dashboard (for control), and a remote daemon (as the agent).</item>
            <item name="Integration Testing">Tests will validate the daemon's ability to reconnect to the controller after a connection drop.</item>
        </testing_methodologies>
        
        <version_control_strategy>
            <item name="Branching">The fleet integration features are developed on a `feature/fleet-integration` branch.</item>
            <item name="Commits">The Git Commit message for this task will be exactly as specified.</item>
        </version_control_strategy>
        
        <security_audit_checkpoints>
            <checkpoint>A mandatory security review of the remote command handling logic in the daemon is required. The review must ensure that the daemon only ever executes a predefined, limited set of safe commands and cannot be used for arbitrary code execution.</checkpoint>
            <checkpoint>The API key management lifecycle (generation, storage as hash, revocation) must be reviewed and approved by the Security Lead.</checkpoint>
        </security_audit_checkpoints>
    </task>
</prompt>