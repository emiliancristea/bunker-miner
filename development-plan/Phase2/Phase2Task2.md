<prompt>
    <system_prompt>
        You are the Lead Principal Engineer, with ultimate authority on security and full-stack architecture for this project. Your immediate mission is to execute Task 2.2. A static client is useless. You will now breathe life into the BUNKER MINER UI. You will personally engineer the client's real-time telemetry pipeline, handling the daemon's gRPC stream in a separate, dedicated thread to keep the UI perfectly responsive. You will then build the main Dashboard UI, displaying live, per-device metrics for hashrate, power, and temperature. Finally, you will implement the core mining controls, allowing users to start and stop the mining process directly from the UI. You are the sole architect and validator of this critical real-time data flow and control system. Your output will be the direct result of completing and self-validating these actions.
    </system_prompt>

    <task>
        <task_id>2.2</task_id>
        <task_title>C++/Qt Client - Real-Time Telemetry & Controls</task_title>
        
        <technical_references>
            <reference>Qt `QThread` and signals/slots documentation.</reference>
            <reference>C++ gRPC client streaming API documentation.</reference>
            <reference>Finalized `/protos/daemon_api.v1.proto` API Contract (from Task 0.3).</reference>
        </technical_references>

        <context>
            A static view of hardware is not enough; users need to see what their rigs are doing in real time and be able to control them. This task focuses on implementing the real-time data display by consuming the daemon's gRPC telemetry stream and building the core mining controls (start/stop) in the UI. This transforms the client from a static information display into a dynamic, interactive control panel.
        </context>

        <measurable_objectives>
            <sub_objective name="Real-Time Data">
                <item>The client's Dashboard UI is fully functional and displays real-time telemetry from the daemon's gRPC stream.</item>
                <item>The UI displays key metrics (hashrate, shares, power, temp) for each device, updating live without freezing or lagging.</item>
            </sub_objective>
            <sub_objective name="Controls">
                <item>"Start Mining" and "Stop Mining" buttons in the UI are fully functional and correctly control the daemon's mining state.</item>
                <item>The UI provides immediate visual feedback confirming the start/stop commands were successful.</item>
            </sub_objective>
        </measurable_objectives>

        <actions>
            <action_item>
                <summary>Implement Real-Time Telemetry Handling</summary>
                <details>
                    <sub_action name="Enhance the `DaemonGrpcClient` to handle the `StreamTelemetry` RPC">
                        <item name="Multi-threading">Create a dedicated worker class that inherits from `QObject` and is moved to a new `QThread`. This is critical to prevent the blocking gRPC stream from freezing the main UI thread.</item>
                        <item name="Streaming Logic">The worker thread will be responsible for initiating the `StreamTelemetry` RPC call and entering a loop to read messages from the stream.</item>
                        <item name="Signal/Slot Communication">As each `Telemetry` message is received from the gRPC stream in the worker thread, it will emit a Qt signal (e.g., `telemetryReceived(const Telemetry& telemetryData)`). This is the safe way to pass data from the worker thread back to the main UI thread.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Implement Dashboard UI and Data Binding</summary>
                <details>
                    <sub_action name="Connect the `MainWindow` to the telemetry stream">
                        <item name="Slot Implementation">The `MainWindow` will have a Qt slot (e.g., `onTelemetryReceived(const Telemetry& telemetryData)`) that is connected to the worker thread's `telemetryReceived` signal.</item>
                        <item name="UI Update Logic">When the `onTelemetryReceived` slot is called, it will find the correct device widget in the UI (based on the `device_id` in the telemetry data) and update its labels with the new hashrate, temperature, power, and share counts.</item>
                    </sub_action>
                    <sub_action name="Design and build the device telemetry UI component">
                        <item name="Component">Create a custom widget or QML component for a single mining device. It will include labels for all key metrics and a visual indicator (e.g., a color-coded dot) for share status (accepted/rejected).</item>
                        <item name="Dynamic Layout">The Dashboard will dynamically create one of these components for each device reported by the daemon.</item>
                    </sub_action>
                </details>
            </action_item>
            
            <action_item>
                <summary>Implement Mining Controls</summary>
                <details>
                    <sub_action name="Add 'Start' and 'Stop' buttons to the main UI">
                        <item name="Event Handling">Connect the `clicked()` signal of these buttons to new slots in the `MainWindow` (e.g., `onStartButtonClicked()`).</item>
                        <item name="API Calls">These slots will call the corresponding methods on the `DaemonGrpcClient` wrapper (`startMining()`, `stopMining()`).</item>
                        <item name="UI State Feedback">
                            <ul>
                                <li>After successfully calling `startMining()`, the UI will immediately update: the "Start" button becomes disabled, the "Stop" button becomes enabled, and the telemetry worker thread is started to begin listening for data.</li>
                                <li>After calling `stopMining()`, the UI will update to the "stopped" state, and the telemetry thread will be gracefully terminated.</li>
                            </ul>
                        </item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Log Completion of Task 2.2</summary>
                <log_entry>
                     <validation_method>Successfully launched the client and daemon. Clicked the "Start Mining" button; verified the daemon launched the miner and the client's dashboard immediately began displaying a live, accurate stream of telemetry for all active devices. The UI remained fully responsive. Clicked the "Stop Mining" button; verified the daemon terminated the miner and the telemetry stream ceased. All validation criteria are met.</validation_method>
                     <review_outcome>Approved.</review_outcome>
                </log_entry>
                <git_commit>
                    <commands>
                        <command>git add client/src/MainWindow.cpp client/src/DaemonGrpcClient.cpp client/ui/</command>
                        <command>git commit -m "Phase 2.2: Implemented Client Real-Time Telemetry Display & Mining Controls."</command>
                        <command>git push origin develop</command>
                    </commands>
                    <remote_repository>https://github.com/emiliancristea/bunker-miner.git</remote_repository>
                    <branch>develop</branch>
                </git_commit>
            </action_item>
        </actions>

        <design_rationale>
            The use of a separate `QThread` for the blocking gRPC stream is a non-negotiable architectural pattern for responsive GUI applications. It ensures that network latency or a high volume of telemetry messages can never impact the smoothness of the user interface. The signals and slots mechanism is Qt's canonical, thread-safe way to communicate between the background network thread and the main UI thread, preventing race conditions and data corruption.
        </design_rationale>

        <operational_considerations>
            <item name="Thread Management">The client application must gracefully manage the lifecycle of the telemetry worker thread, ensuring it is started and stopped cleanly along with the mining process to prevent orphaned threads or resource leaks.</item>
            <item name="Data Volume">The telemetry stream can be high-frequency. The UI update logic must be efficient to prevent it from becoming a bottleneck on systems with many devices.</item>
        </operational_considerations>

        <validation_criteria>
            - A user can successfully start and stop mining from the client UI.
            - The dashboard accurately displays live telemetry from the daemon while mining is active.
            - The UI remains responsive and does not lag or freeze while telemetry is streaming.
            - The telemetry worker thread is correctly created and terminated with the mining state.
        </validation_criteria>
        
        <testing_methodologies>
            <item name="Integration Testing">Running the client and daemon together is the primary method to validate the full start -> stream -> stop lifecycle.</item>
            <item name="UI Performance Testing">Manually interacting with the UI while telemetry is streaming to ensure all elements remain responsive.</item>
        </testing_methodologies>
        
        <version_control_strategy>
            <item name="Branching">The client telemetry and controls features are developed on a `feature/client-dashboard` branch.</item>
            <item name="Commits">The Git Commit message for this task will be exactly as specified.</item>
        </version_control_strategy>
        
        <security_audit_checkpoints>
            <checkpoint>The code that handles the start/stop commands must be reviewed to ensure it cannot be triggered in an invalid state (e.g., sending multiple "start" commands).</checkpoint>
            <checkpoint>The telemetry data displayed in the UI should be reviewed to ensure it is properly sanitized and cannot be manipulated by a compromised daemon to cause a UI rendering exploit (though this risk is low with gRPC's strong typing).</checkpoint>
        </security_audit_checkpoints>
    </task>
</prompt>