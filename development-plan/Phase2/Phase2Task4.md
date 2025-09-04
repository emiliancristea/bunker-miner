<prompt>
    <system_prompt>
        You are the Lead Principal Engineer, with ultimate authority on security and full-stack architecture for this project. Your immediate mission is to execute Task 2.4. All major components for the single-rig MVP are now implemented. You will now weave them into a cohesive whole. You will personally integrate the profit-switching controls into the C++/Qt client, allowing users to activate and monitor the automated intelligence. You will then build a simple, secure, local-only web dashboard with a WebSocket backend to provide at-a-glance monitoring for headless rigs. Finally, you will conduct the comprehensive integration testing campaign for all of Phase 2, formalize the deliverable, and give the final sign-off, confirming that our single-rig product is feature-complete and stable. Your output will be the direct result of completing and self-validating these actions.
    </system_prompt>

    <task>
        <task_id>2.4</task_id>
        <task_title>Integration Testing, Web Dashboard & Phase 2 Deliverable</task_title>
        
        <technical_references>
            <reference>`axum` and `tokio-tungstenite` crate documentation (for Web Dashboard).</reference>
            <reference>All Phase 2 Test Plans.</reference>
            <reference>Finalized v0.1 `daemon_api.v1.proto` Contract.</reference>
        </technical_references>

        <context>
            All major components for Phase 2—the C++ client, the real-time telemetry pipeline, and the Rust profit-switching engine—have been implemented. This final task focuses on integrating them into a single, polished product. We will add the final UI controls for the profit engine, provide a simple web interface for headless rigs, and conduct a comprehensive integration test to validate the entire system.
        </context>

        <measurable_objectives>
            <sub_objective name="Integration">
                <item>The C++/Qt client is fully integrated with the profit-switching engine, allowing users to enable "auto" mode and view real-time profitability data.</item>
                <item>A simple, local-only web dashboard is created and functional, displaying the same real-time telemetry as the desktop client.</item>
            </sub_objective>
            <sub_objective name="Validation & Project Management">
                <item>A full end-to-end integration test of the complete single-rig application passes successfully.</item>
                <item>The Phase 2 Deliverable is formally documented and signed off by all stakeholders.</item>
            </sub_objective>
        </measurable_objectives>

        <actions>
            <action_item>
                <summary>Integrate Profit Engine into C++/Qt Client</summary>
                <details>
                    <sub_action name="Enhance the client UI and gRPC client service">
                        <item name="Profitability UI">Implement a new page or a dedicated section in the Dashboard that calls the daemon's `GetProfitability` gRPC endpoint. Display the returned data in a clear, sorted table showing the current profitability of all benchmarked algorithms.</item>
                        <item name="Auto-Mode Control">Add a primary toggle switch or a prominent "Start Auto-Mining" button to the main UI. This control will call a newly added `StartMining` gRPC endpoint with an `auto_mode = true` parameter, instructing the daemon to activate the profit-switching logic.</item>
                        <item name="Status Display">The UI must clearly indicate when it is in "auto" mode and display the currently active algorithm, which will change dynamically.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Implement Local Web Dashboard</summary>
                <details>
                    <sub_action name="Add a lightweight web server to the Rust daemon">
                        <item name="Web Server">Use the `axum` crate to add an HTTP server that runs alongside the gRPC server. It must bind to `localhost` by default for security.</item>
                        <item name="WebSocket Backend">Add a `/ws` endpoint using `tokio-tungstenite`. This WebSocket will be connected to the same central telemetry `Broadcaster` as the gRPC stream (from Task 1.3).</item>
                        <item name="Frontend">Create a simple, single-page HTML file with minimal JavaScript. The JavaScript will connect to the `/ws` endpoint and use the received JSON telemetry messages to update the values in a simple table, providing a live view of the rig's status.</item>
                        <item name="Security">The web server must be configured to prevent Cross-Site WebSocket Hijacking (CSWSH) by validating the `Origin` header of incoming WebSocket upgrade requests.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Conduct Final Integration Testing and Formalize Deliverable</summary>
                <details>
                    <sub_action name="Execute the full end-to-end integration test scenario">
                        <item name="Test Scenario">
                            <ol>
                                <li>Launch the daemon and the C++/Qt client.</li>
                                <li>From the client, start the daemon in "auto" profit-switching mode.</li>
                                <li>Open a web browser to the local web dashboard URL (e.g., `http://localhost:4067`).</li>
                                <li>Using a mock API server, manipulate the profitability data to trigger a switch.</li>
                                <li>**Verify:** The daemon correctly switches miners. Both the desktop client and the web dashboard accurately reflect the change in algorithm and the new, live telemetry data.</li>
                            </ol>
                        </item>
                    </sub_action>
                    <sub_action name="Document and approve the formal Phase 2 Deliverable">
                        <item name="Deliverable Definition">"A feature-complete mining orchestration application for single rigs, featuring an intelligent profit-switching engine, a polished C++/Qt desktop client for management, and a local web dashboard for monitoring. The system supports stable, automated mining on Windows and Linux for NVIDIA, AMD, and CPU devices."</item>
                        <item name="Closure Meeting">Conduct a Phase 2 closure review meeting to demonstrate the final product and obtain formal sign-off from all stakeholders.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Log Completion of Task 2.4</summary>
                <log_entry>
                     <validation_method>Successfully executed the full end-to-end integration test. Started the client, enabled auto-profit mode, and verified that both the Qt client and the new web dashboard correctly reflected live telemetry. Using a mock market data API, successfully triggered an automatic algorithm switch and verified that both UIs updated to show the new state. The Phase 2 deliverable was demonstrated and formally signed off. All validation criteria are met.</validation_method>
                     <review_outcome>Phase 2 Approved for Closure.</review_outcome>
                </log_entry>
                <git_commit>
                    <commands>
                        <command>git add daemon/src/web_dashboard.rs daemon/src/main.rs client/src/MainWindow.cpp client/web/</command>
                        <command>git commit -m "Phase 2.4: Full Integration Testing Passed; Web Dashboard Implemented; Phase 2 Complete."</command>
                        <command>git push origin develop</command>
                    </commands>
                    <remote_repository>https://github.com/emiliancristea/bunker-miner.git</remote_repository>
                    <branch>develop</branch>
                </git_commit>
            </action_item>
        </actions>

        <design_rationale>
            The final integration of the profit engine into the UI completes the core user journey and delivers on the main promise of the application. The addition of a local web dashboard is a low-cost, high-value feature. It provides a crucial monitoring interface for users who run BUNKER MINER on headless server rigs or mining farms, where a full desktop GUI is not practical. This makes the application dramatically more versatile.
        </design_rationale>

        <operational_considerations>
            <item name="Resource Usage">The daemon now runs both a gRPC server and a web/WebSocket server. The idle resource consumption of these components must be profiled to ensure the daemon remains lightweight.</item>
            <item name="User Experience">The clarity of the UI in conveying the state of the profit-switching engine is paramount. Users must be able to understand at a glance what the system is doing and why.</item>
        </operational_considerations>

        <validation_criteria>
            - A user can enable auto-profit mode from the C++/Qt client, and the daemon responds accordingly.
            - The client correctly displays real-time profitability rankings.
            - The local web dashboard successfully connects to the daemon and displays live telemetry.
            - The full E2E integration test for profit switching and UI feedback passes.
            - The Phase 2 Deliverable is formally signed off.
        </validation_criteria>
        
        <testing_methodologies>
            <item name="End-to-End (E2E) System Testing">The primary validation is a full test of the entire application stack: the daemon (with its profit engine) and the C++ client, all interacting together.</item>
            <item name="UI Testing">Manually testing both the Qt client and the web dashboard to ensure data is displayed correctly and controls work as expected.</item>
        </testing_methodologies>
        
        <version_control_strategy>
            <item name="Branching">The final integration work is developed on a `feature/p2-integration` branch.</item>
            <item name="Commits">The Git Commit message for this task will be exactly as specified.</item>
        </version_control_strategy>
        
        <security_audit_checkpoints>
            <checkpoint>The Security Lead must review the WebSocket server implementation, specifically the `Origin` header validation, to prevent Cross-Site WebSocket Hijacking (CSWSH).</checkpoint>
            <checkpoint>The new gRPC endpoints for profitability must be reviewed to ensure they do not disclose sensitive information and are covered by the existing security model.</checkpoint>
        </security_audit_checkpoints>
    </task>
</prompt>