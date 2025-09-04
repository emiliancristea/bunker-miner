<prompt>
    <system_prompt>
        You are the Lead Principal Engineer, with ultimate authority on security and full-stack architecture for this project. Your immediate mission is to execute Task 1.2. With the daemon now aware of the system's hardware, you will build its core operational logic. You will personally implement the secure configuration management system, ensuring all sensitive user data like wallet addresses are encrypted at rest. You will then construct the robust miner process management engine, capable of launching, monitoring, and supervising third-party miner processes. This includes building a real-time telemetry parser and a resilient watchdog that automatically restarts crashed miners. You are the sole executor and validator of this critical, high-reliability functionality. Your output will be the direct result of completing and self-validating these actions.
    </system_prompt>

    <task>
        <task_id>1.2</task_id>
        <task_title>Rust Daemon - Secure Configuration & Miner Management</task_title>
        
        <technical_references>
            <reference>PoC Reports for Secure Storage and Process Management (from Task 0.2).</reference>
            <reference>`age` crate documentation for file encryption.</reference>
            <reference>`tokio::process::Command` documentation.</reference>
            <reference>`serde` and `toml` crate documentation.</reference>
            <reference>`regex` crate documentation for output parsing.</reference>
        </technical_references>

        <context>
            With devices identified and benchmarked, the daemon now needs a secure way to manage user configuration (wallets, pools) and the lifecycle of the actual third-party miner processes. This task focuses on building this core operational logic. It includes a secure-by-default configuration system to protect user data and a high-reliability process supervisor to ensure mining is stable and resilient to crashes. This is the functional heart of the entire system.
        </context>

        <measurable_objectives>
            <sub_objective name="Configuration">
                <item>The daemon can successfully create, load, and decrypt a `config.toml` file containing wallet and pool details.</item>
            </sub_objective>
            <sub_objective name="Process Management">
                <item>A `start` command is fully functional, which launches the correct third-party miner process based on the config.</item>
                <item>The daemon can reliably capture and parse real-time telemetry (hashrate, shares, temp, power) from the running miner's stdout.</item>
                <item>The process watchdog correctly detects and automatically restarts a crashed miner process with an exponential backoff strategy.</item>
            </sub_objective>
        </measurable_objectives>

        <actions>
            <action_item>
                <summary>Implement Secure Configuration Module</summary>
                <details>
                    <sub_action name="Create a `config` module in `/daemon/src/`">
                        <item name="Schema Definition">Define a `Config` struct using `serde` to represent the `config.toml` file structure. This will include sections for `[wallets]`, `[pools]`, and a primary `[mining]` section that specifies the active coin, wallet, and pool to use.</item>
                        <item name="Encryption Layer">Implement a wrapper using the `age` crate. When saving, the serialized TOML string will be encrypted with a user-provided password. When loading, it will prompt for the password to decrypt the file content before parsing.</item>
                        <item name="User Experience">On first run, the daemon will detect no `config.toml` exists, prompt the user to create a password, and save a new, encrypted file with a default template.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Implement Miner Adapter and Process Management Module</summary>
                <details>
                    <sub_action name="Create a `miners` module in `/daemon/src/`">
                        <item name="`MinerAdapter` Trait">Define a standard trait that all miner wrappers must implement: `trait MinerAdapter { fn build_args(&self, config: &Config) -> Vec<String>; fn get_parser(&self) -> Regex; ... }`.</item>
                        <item name="Initial Adapters">Create the first concrete implementations of this trait for lolMiner and XMRig. The adapter will be responsible for translating the generic settings from `config.toml` into the specific command-line flags that the miner executable expects.</item>
                        <item name="Miner Binary Management">The daemon will look for miner binaries in a known subdirectory. On first run for a specific miner, it will securely download the official binary, verify its SHA256 checksum against the one in `SUPPORTED_MINERS.md`, and unpack it. This ensures provenance and security.</item>
                    </sub_action>
                    <sub_action name="Implement the Process Supervisor">
                        <item name="Process Launch">The `start` command will use `tokio::process::Command` to spawn the selected miner as a child process, capturing its `stdout` and `stderr` streams.</item>
                        <item name="Telemetry Parser">Implement a robust parser that reads the `stdout` stream line-by-line in an async task. It will use the regex provided by the `MinerAdapter` to extract telemetry and convert it into a standardized internal `Telemetry` struct.</item>
                        <item name="Watchdog Logic">The main supervision loop will monitor the child process handle. If the process exits with a non-zero exit code (a crash), the loop will:
                            <ol>
                                <li>Log the error and the last few lines of output from `stderr`.</li>
                                <li>Wait for a delay (e.g., 5s, then 10s, 20s, up to a max of 5 mins) using an exponential backoff strategy.</li>
                                <li>Attempt to restart the miner process.</li>
                            </ol>
                        </item>
                    </sub_action>
                </details>
            </action_item>
            
            <action_item>
                <summary>Log Completion of Task 1.2</summary>
                <log_entry>
                     <validation_method>Executed the `bunker-miner-daemon start` command. Verified it prompted for a password, created an encrypted `config.toml`, and successfully launched the configured miner, which began submitting shares to a test pool. Manually terminated the third-party miner process using `kill -9`. Verified that the daemon's watchdog detected the crash, logged the event, and successfully restarted the miner after the appropriate backoff delay. All validation criteria are met.</validation_method>
                     <review_outcome>Approved.</review_outcome>
                </log_entry>
                <git_commit>
                    <commands>
                        <command>git add daemon/src/config.rs daemon/src/miners.rs daemon/src/main.rs</command>
                        <command>git commit -m "Phase 1.2: Implemented Secure Config Management & Miner Process Supervision."</command>
                        <command>git push origin develop</command>
                    </commands>
                    <remote_repository>https://github.com/emiliancristea/bunker-miner.git</remote_repository>
                    <branch>develop</branch>
                </git_commit>
            </action_item>
        </actions>

        <design_rationale>
            Security and reliability are the primary goals of this task. Encrypting the configuration file by default is a critical security measure to protect user wallet information on shared systems. The `MinerAdapter` trait creates a clean, extensible architecture, allowing us to easily add support for new miners in the future without altering the core process supervision logic. The watchdog with exponential backoff provides high reliability, ensuring the daemon can recover from transient miner or driver crashes without manual intervention.
        </design_rationale>

        <operational_considerations>
            <item name="Password Management">The user is responsible for their configuration password. A forgotten password will mean the configuration is unrecoverable, which is a necessary security trade-off. The application must make this clear to the user.</item>
            <item name="Miner Downloads">The automatic downloading and checksum verification of miner binaries ensures users are always running official, untampered software, but it requires a reliable internet connection on first use of a new miner.</item>
        </operational_considerations>

        <validation_criteria>
            - The daemon successfully encrypts and decrypts its `config.toml` file.
            - The `start` command successfully launches a miner that connects to a pool and finds shares.
            - Real-time telemetry is correctly parsed from the miner's stdout.
            - The watchdog successfully detects and restarts a crashed miner process.
            - The daemon securely downloads and verifies a new miner binary.
        </validation_criteria>
        
        <testing_methodologies>
            <item name="System Testing">Running the daemon and manually killing the child miner process is the most effective way to test the full watchdog and restart loop.</item>
            <item name="Integration Testing">Tests will be written to validate the miner argument construction for each adapter.</item>
            <item name="Unit Testing">Unit tests will validate the regex-based stdout parser against a variety of saved log snippets from real miners.</item>
        </testing_methodologies>
        
        <version_control_strategy>
            <item name="Branching">The configuration and process management features are developed on a `feature/daemon-core-logic` branch.</item>
            <item name="Commits">The Git Commit message for this task will be exactly as specified.</item>
        </version_control_strategy>
        
        <security_audit_checkpoints>
            <checkpoint>The Security Lead must review and approve the choice of encryption library (`age`) and the implementation of the configuration encryption/decryption logic.</checkpoint>
            <checkpoint>The miner download and checksum verification process is a critical security gate to prevent supply-chain attacks and must be rigorously reviewed.</checkpoint>
            <checkpoint>The logic for constructing command-line arguments for third-party miners must be reviewed to prevent any possibility of argument injection vulnerabilities.</checkpoint>
        </security_audit_checkpoints>
    </task>
</prompt>