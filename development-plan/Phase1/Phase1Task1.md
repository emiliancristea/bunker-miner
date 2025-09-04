<prompt>
    <system_prompt>
        You are the Lead Principal Engineer, with ultimate authority on security and full-stack architecture for this project. Your immediate mission is to execute Task 1.1. You will now build the sensory organs of the BUNKER MINER daemon. You will personally implement the hardware detection and capability-assessment engine in Rust, using the PoC-validated libraries. You will reliably identify all supported NVIDIA, AMD, and CPU hardware, gather their vital statistics, and engineer a non-destructive benchmarking system to establish a baseline performance profile for each available mining algorithm on each device. This data is the foundation of our future profit-switching intelligence. You are the sole executor and validator of this core functionality. Your output will be the direct result of completing and self-validating these actions.
    </system_prompt>

    <task>
        <task_id>1.1</task_id>
        <task_title>Rust Daemon - Device Detection & Benchmarking Engine</task_title>
        
        <technical_references>
            <reference>PoC Reports for GPU/CPU detection libraries (from Task 0.2).</reference>
            <reference>`nvml-wrapper`, `rocm-smi`, `sysinfo` crate documentation.</reference>
            <reference>`clap` crate documentation for command-line parsing.</reference>
            <reference>`serde` crate documentation for JSON serialization.</reference>
        </technical_references>

        <context>
            The daemon cannot manage what it cannot see. The first functional step is to build a robust hardware detection and capability-assessment module. This module must reliably identify all supported GPUs and CPUs, gather their vital statistics (VRAM, driver versions), and establish a baseline performance profile for each available mining algorithm. This benchmark data is the foundational dataset that the profit-switching engine will rely on in later phases.
        </context>

        <measurable_objectives>
            <sub_objective name="Hardware Detection">
                <item>The daemon can successfully detect and enumerate all supported NVIDIA and AMD GPUs, and the primary CPU, on both Windows and Linux.</item>
                <item>The daemon can retrieve key static information (Name, Driver Version, VRAM) for each device.</item>
            </sub_objective>
            <sub_objective name="Benchmarking">
                <item>A new `benchmark` command is fully functional in the daemon's CLI.</item>
                <item>The command correctly runs a quick, non-destructive performance test for each supported algorithm on each detected device.</item>
                <item>Benchmark results, including hashrate and average power draw, are correctly calculated and saved to a local `profiles.json` file.</item>
            </sub_objective>
        </measurable_objectives>

        <actions>
            <action_item>
                <summary>Implement Hardware Detection Module</summary>
                <details>
                    <sub_action name="Create a `hardware` module in `/daemon/src/`">
                        <item name="NVIDIA Detection">Use the `nvml-wrapper` crate to connect to the NVIDIA driver, enumerate GPUs, and query device handles, names, driver versions, and total VRAM.</item>
                        <item name="AMD Detection">Use a wrapper around the `rocm-smi` command-line tool or the `adl` crate to enumerate AMD GPUs and query their static information.</item>
                        <item name="CPU Detection">Use the `sysinfo` crate to get the CPU brand, core count, and name.</item>
                        <item name="Abstraction">Create a unified `MiningDevice` struct that can represent any of these hardware types, abstracting away the platform-specific details.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Implement Benchmarking Engine</summary>
                <details>
                    <sub_action name="Create a `benchmarking` module in `/daemon/src/`">
                        <item name="Algorithm Mapping">Create a mapping that defines which algorithms are supported by which third-party miners (e.g., lolMiner supports kHeavyHash and Etchash).</item>
                        <item name="Benchmark Runner">Implement the core logic that iterates through each detected `MiningDevice` and its supported algorithms.</item>
                        <item name="Process Execution">For each device/algorithm pair, launch the appropriate miner process (e.g., `lolMiner.exe --algo KASPA --benchmark BENCHMARK_ID`) for a fixed, short duration (e.g., 60 seconds).</item>
                        <item name="Data Capture">While the benchmark is running, simultaneously poll the `hardware` module for power usage. Parse the miner's stdout at the end of the run to extract the final average hashrate.</item>
                    </sub_action>
                </details>
            </action_item>
            
            <action_item>
                <summary>Implement Profile Management and CLI Command</summary>
                <details>
                    <sub_action name="Create a `profiles` module in `/daemon/src/`">
                        <item name="Data Structure">Define a `DeviceProfile` struct using `serde` that can store a map of algorithm names to their benchmark results (`hashrate_mhs`, `power_watts`).</p>
                        <item name="Persistence">Implement functions to save and load a `Vec<DeviceProfile>` to and from a `profiles.json` file in the daemon's configuration directory.</item>
                    </sub_action>
                    <sub_action name="Implement the daemon's CLI using `clap`">
                        <item name="`benchmark` Command">Create a new subcommand, `bunker-miner-daemon benchmark`, which triggers the full benchmarking process and saves the results.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Log Completion of Task 1.1</summary>
                <log_entry>
                     <validation_method>Executed the `bunker-miner-daemon benchmark` command on dedicated test rigs featuring NVIDIA (Windows), AMD (Linux), and CPU hardware. Verified that all devices were correctly identified. Inspected the resulting `profiles.json` file to confirm that all relevant algorithms were benchmarked and that the stored hashrate and power draw figures were accurate and reasonable. All validation criteria are met.</validation_method>
                     <review_outcome>Approved.</review_outcome>
                </log_entry>
                <git_commit>
                    <commands>
                        <command>git add daemon/src/hardware.rs daemon/src/benchmarking.rs daemon/src/profiles.rs daemon/src/main.rs</command>
                        <command>git commit -m "Phase 1.1: Implemented Rust Daemon Device Detection & Benchmarking Engine."</command>
                        <command>git push origin develop</command>
                    </commands>
                    <remote_repository>https://github.com/emiliancristea/bunker-miner.git</remote_repository>
                    <branch>develop</branch>
                </git_commit>
            </action_item>
        </actions>

        <design_rationale>
            Accurate hardware detection and benchmarking form the data-driven foundation for all intelligent features of BUNKER MINER. By abstracting the hardware-specific query logic into a single module, we isolate platform-dependent code and create a clean interface for the rest of the application. The on-demand benchmarking process ensures that the profit-switching engine will always operate on data that is specific to the user's actual hardware and driver configuration, leading to more accurate profitability calculations.
        </design_rationale>

        <operational_considerations>
            <item name="Privileges">Querying hardware telemetry, especially on Linux, may require the daemon to be run with specific user group permissions (e.g., `render` or `video` group) to access the GPU driver interfaces.</item>
            <item name="Benchmark Time">The initial benchmark run can be time-consuming. The UI/CLI must provide clear feedback to the user that this is a one-time setup process. The results will be cached for all future runs.</item>
        </operational_considerations>

        <validation_criteria>
            - The `bunker-miner-daemon benchmark` command successfully runs to completion on test rigs with NVIDIA, AMD, and CPU hardware.
            - The command produces a `profiles.json` file containing accurate hashrate and power data for all supported device/algorithm combinations.
            - The detection logic correctly identifies device names, VRAM, and driver versions.
            - The code is peer-reviewed and signed off.
        </validation_criteria>
        
        <testing_methodologies>
            <item name="System Testing">The primary validation method is running the final compiled daemon binary on a variety of physical hardware configurations to test the detection and benchmarking logic in a real-world environment.</item>
            <item name="Unit Testing">Unit tests will be written for the logic that parses benchmark output from miner stdout, using saved text fixtures.</item>
        </testing_methodologies>
        
        <version_control_strategy>
            <item name="Branching">The device detection and benchmarking engine is developed on a `feature/daemon-hardware-detection` branch.</item>
            <item name="Commits">The Git Commit message for this task will be exactly as specified.</item>
        </version_control_strategy>
        
        <security_audit_checkpoints>
            <checkpoint>The code that launches third-party miner processes for benchmarking must be reviewed to ensure it uses sanitized inputs and cannot be exploited for arbitrary code execution.</checkpoint>
            <checkpoint>Review the permissions required for the daemon to access hardware telemetry to ensure it adheres to the principle of least privilege.</checkpoint>
        </security_audit_checkpoints>
    </task>
</prompt>