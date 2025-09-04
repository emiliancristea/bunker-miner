<prompt>
    <system_prompt>
        You are the Lead Principal Engineer, with ultimate authority on security and full-stack architecture for this project. Your immediate mission is to execute Task 2.1. You will now construct the native desktop experience for BUNKER MINER. You will personally build the foundational structure of the C++/Qt desktop application, including the main window and core UI placeholders. Your most critical responsibility in this task is to engineer the "client backend"—a robust C++ service that flawlessly integrates with the Rust daemon's gRPC API. This bridge is the central nervous system of the client application, and you are its sole architect and validator. Your output will be the direct result of completing and self-validating these actions.
    </system_prompt>

    <task>
        <task_id>2.1</task_id>
        <task_title>C++/Qt Client - Core Application Shell & Daemon Integration</task_title>
        
        <technical_references>
            <reference>Qt 6 and CMake Documentation.</reference>
            <reference>C++ gRPC Library Documentation.</reference>
            <reference>Finalized `/protos/daemon_api.v1.proto` API Contract (from Task 0.3).</reference>
        </technical_references>

        <context>
            The daemon provides the core mining functionality, but it requires a user-friendly front end for management and monitoring. This task involves building the foundational structure of the C++/Qt desktop application. This includes creating the main application window, setting up the basic UI navigation structure, and, most critically, implementing the client-side gRPC service that will communicate with the Rust daemon. This establishes the skeleton of the application, ready for real-time features to be added.
        </context>

        <measurable_objectives>
            <sub_objective name="Application Shell">
                <item>A core C++/Qt application shell is created, which successfully compiles and launches a main window.</item>
                <item>The UI contains placeholders for the main navigation sections (e.g., Dashboard, Benchmarks, Settings).</item>
            </sub_objective>
            <sub_objective name="Daemon Integration">
                <item>A C++ "gRPC client service" is implemented that can successfully connect to the Rust daemon's gRPC API.</item>
                <item>On startup, the client application can successfully fetch the static system info (GPUs/CPUs) from the daemon and display it within the UI.</item>
                <item>The UI correctly displays a "Disconnected" or "Error" state if the daemon is not running.</item>
            </sub_objective>
        </measurable_objectives>

        <actions>
            <action_item>
                <summary>Initialize C++/Qt Application Project</summary>
                <details>
                    <sub_action name="Set up the C++/Qt project in `/client` using CMake">
                        <item name="Project Structure">Create the `main.cpp` entry point, a `MainWindow` class, and the associated UI files (`.ui` or QML).</item>
                        <item name="Build System">Configure `CMakeLists.txt` to correctly find and link the Qt 6 libraries and to handle UI file compilation (`uic`).</item>
                        <item name="UI Placeholders">Design the main window with a simple navigation structure (e.g., a `QStackedWidget` or `QTabWidget`) and create empty placeholder widgets for the `Dashboard`, `Benchmarks`, and `Settings` pages.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Implement gRPC Client Integration</summary>
                <details>
                    <sub_action name="Generate C++ gRPC Stub Code">
                        <item name="Build Integration">Update the `CMakeLists.txt` file to find the `daemon_api.v1.proto` file and use the `protoc` compiler with the gRPC plugin to generate the C++ client stub code during the build process.</item>
                    </sub_action>
                    <sub_action name="Implement a `DaemonGrpcClient` C++ class">
                        <item name="Wrapper Logic">This class will encapsulate all gRPC interactions. It will manage the gRPC channel and provide clean, synchronous or asynchronous C++ methods that wrap the raw gRPC calls (e.g., `std::vector<DeviceInfo> getSystemInfo()`).</item>
                        <item name="Thread Safety">Ensure the client is designed to be safely called from the Qt main UI thread.</item>
                    </sub_action>
                </details>
            </action_item>
            
            <action_item>
                <summary>Connect UI to Daemon</summary>
                <details>
                    <sub_action name="Integrate the `DaemonGrpcClient` with the `MainWindow`">
                        <item name="Instantiation">The `MainWindow` will create an instance of the `DaemonGrpcClient` upon construction.</item>
                        <item name="Initial Data Fetch">In the `MainWindow`'s startup logic, it will call the `getSystemInfo()` method on the gRPC client.</item>
                        <item name="UI Population">The list of `DeviceInfo` objects returned from the daemon will be used to populate a "Devices" list widget in the UI.</item>
                        <item name="Error Handling">Implement robust error handling for the gRPC call. If the connection to the daemon fails, the UI must display a clear and persistent "Daemon connection failed. Is BUNKER MINER running?" status message.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Log Completion of Task 2.1</summary>
                <log_entry>
                     <validation_method>Successfully compiled and launched the C++/Qt client on both Windows and Linux. On startup, with the daemon running, the client correctly connected via gRPC and displayed the list of GPUs and CPU from the test rig. Terminated the daemon process and relaunched the client; verified that the UI correctly displayed a prominent "Daemon not detected" error state. All validation criteria are met.</validation_method>
                     <review_outcome>Approved.</review_outcome>
                </log_entry>
                <git_commit>
                    <commands>
                        <command>git add client/ client/main.cpp client/CMakeLists.txt client/src/</command>
                        <command>git commit -m "Phase 2.1: Implemented C++/Qt Client Shell & gRPC Daemon Integration."</command>
                        <command>git push origin develop</command>
                    </commands>
                    <remote_repository>https://github.com/emiliancristea/bunker-miner.git</remote_repository>
                    <branch>develop</branch>
                </git_commit>
            </action_item>
        </actions>

        <design_rationale>
            A clean separation between the UI (`MainWindow`) and the communication logic (`DaemonGrpcClient`) is a fundamental aspect of maintainable GUI application design. This allows the UI to be tested independently with a mock client, and the client logic to be tested without a UI. Using the auto-generated gRPC stubs enforces the API contract, eliminating an entire class of potential bugs related to manual data serialization and API mismatches.
        </design_rationale>

        <operational_considerations>
            <item name="Daemon Dependency">The client is now fundamentally dependent on the daemon. The user experience for when the daemon is not running, or crashes, must be clear and helpful.</item>
            <item name="Build Complexity">The C++ build system is now more complex, as it includes a code generation step for the gRPC stubs. This must be robust and work cross-platform.</item>
        </operational_considerations>

        <validation_criteria>
            - The C++/Qt client successfully launches and displays its main window.
            - The client connects to the running daemon and correctly fetches and displays the system's hardware information.
            - If the daemon is not running, the client displays a clear and correct error/disconnected status.
            - The gRPC C++ stub code is successfully generated as part of the CMake build process.
        </validation_criteria>
        
        <testing_methodologies>
            <item name="Integration Testing">The primary validation is an integration test: running the compiled client and the compiled daemon together to verify they can communicate correctly.</item>
            <item name="UI Testing">Manually testing the UI to ensure the hardware information is displayed correctly and that the error state is handled gracefully.</item>
        </testing_methodologies>
        
        <version_control_strategy>
            <item name="Branching">The initial client application is developed on a `feature/client-shell` branch.</item>
            <item name="Commits">The Git Commit message for this task will be exactly as specified.</item>
        </version_control_strategy>
        
        <security_audit_checkpoints>
            <checkpoint>The gRPC connection logic must be reviewed to ensure it is configured to connect to `localhost` by default, preventing the client from being tricked into connecting to a malicious remote daemon without explicit user configuration.</checkpoint>
            <checkpoint>Error messages displayed in the UI must be reviewed to ensure they do not leak any sensitive information (e.g., detailed connection-refused errors).</checkpoint>
        </security_audit_checkpoints>
    </task>
</prompt>