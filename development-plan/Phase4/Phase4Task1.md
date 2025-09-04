<prompt>
    <system_prompt>
        You are the Lead Principal Engineer, with ultimate authority on security and full-stack architecture for this project. Your immediate mission is to execute Task 4.1. You will now build one of BUNKER MINER's key competitive advantages: the Adaptive Overclocking and Power Tuning Engine. You will personally implement the low-level hardware control logic within the Rust daemon, enabling it to modify GPU clocks and power limits. Your highest priority is safety and security. This feature must be disabled by default, placed behind an "expert mode" toggle with clear warnings, and require elevated privileges to run. You will engineer the system to apply specific, per-algorithm OC profiles and, most critically, to fail safe by reverting to default settings. You are the sole executor and validator of this powerful and high-risk functionality. Your output will be the direct result of completing and self-validating these actions.
    </system_prompt>

    <task>
        <task_id>4.1</task_id>
        <task_title>Rust Daemon - Adaptive Overclocking & Power Tuning Engine</task_title>
        
        <technical_references>
            <reference>Finalized ADR for Adaptive Overclocking Engine Design.</reference>
            <reference>`nvml-wrapper` crate documentation for setting clocks/power.</reference>
            <reference>`rocm-smi` CLI documentation or `adl` crate for AMD control.</reference>
            <reference>Windows and Linux privilege escalation guides (e.g., UAC manifests, sudo).</reference>
        </technical_references>

        <context>
            Static overclock profiles are inefficient. Different mining algorithms stress a GPU's core and memory differently. A truly intelligent miner should adapt its hardware profile to match the algorithm, maximizing the hashrate-to-power efficiency. This task involves building a sophisticated engine within the daemon to manage and apply per-algorithm overclocking and undervolting profiles. Due to the inherent risk of modifying hardware settings, this feature must be built with a security- and safety-first mindset.
        </context>

        <measurable_objectives>
            <sub_objective name="Hardware Control">
                <item>The Rust daemon can successfully apply core clock offsets, memory clock offsets, and power limits to supported NVIDIA and AMD GPUs on both Windows and Linux.</item>
            </sub_objective>
            <sub_objective name="Integration">
                <item>The `config.toml` is enhanced to support defining specific OC profiles for each mining algorithm.</item>
                <item>When the profit-switcher changes algorithms, the daemon automatically and correctly applies the corresponding OC profile.</item>
                <item>When mining stops for any reason, all hardware settings are guaranteed to revert to their default state.</item>
            </sub_objective>
            <sub_objective name="Security & Safety">
                <item>All overclocking functionality is disabled by default and can only be activated via an explicit "expert mode" toggle in the configuration, which triggers a clear warning in the UI.</item>
                <item>The daemon correctly requests and handles the elevated privileges required for this functionality.</item>
            </sub_objective>
        </measurable_objectives>

        <actions>
            <action_item>
                <summary>Implement Hardware Control Module</summary>
                <details>
                    <sub_action name="Create an `overclock` module in `/daemon/src/`">
                        <item name="Security First Implementation">
                            <ul>
                                <li>The entire module's functionality will be gated by a feature flag in the code and a runtime check for `[features].enable_overclocking = true` in `config.toml`. If false, all functions will immediately return a "disabled" error.</li>
                                <li>Implement privilege handling. The daemon must detect if it is running with the required admin/root privileges and refuse to enable the OC engine if it is not. On Windows, this requires an embedded manifest to trigger a UAC prompt.</li>
                            </ul>
                        </item>
                        <item name="Platform-Specific Integration">
                            <ul>
                                <li>**NVIDIA:** Use the `nvml-wrapper` crate to call the necessary functions for setting clock offsets and power limits.</li>
                                <li>**AMD:** Use `std::process::Command` to call the `rocm-smi` CLI tool with the appropriate arguments, or use the `adl` crate.</li>
                            </ul>
                        </item>
                        <item name="Failsafe Logic">Implement a `revert_to_defaults()` function that is guaranteed to be called when the mining process stops, whether gracefully (user command) or due to a crash. This can be achieved using RAII guards on the main mining loop.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Integrate with Configuration and Profit Engine</summary>
                <details>
                    <sub_action name="Enhance the `config.toml` schema">
                        <item name="`[overclock_profiles]` Section">Allow users to define profiles for specific algorithms, e.g., `[overclock_profiles.kHeavyHash]`. Each profile can contain `core_clock_offset`, `memory_clock_offset`, `power_limit_watts`.</item>
                    </sub_action>
                    <sub_action name="Integrate with the Miner Management and Profit Switching Logic">
                        <item name="Pre-Mining Hook">Before the daemon starts a new miner process, it will first check if an OC profile exists for the target algorithm. If so, it will call the `overclock` module to apply the settings.</item>
                        <item name="Post-Mining Hook">The `revert_to_defaults()` failsafe must be triggered immediately upon stopping the miner.</item>
                    </sub_action>
                </details>
            </action_item>
            
            <action_item>
                <summary>Enhance API and Client UI</summary>
                <details>
                    <sub_action name="Update the daemon's gRPC API">
                        <item name="New RPCs">Add endpoints to `ListOCProfiles`, `ApplyOCProfile`, and `GetHardwareDefaults`.</item>
                    </sub_action>
                    <sub_action name="Update the C++/Qt client's 'Settings' page">
                        <item name="Expert Mode Toggle">Add a toggle to enable overclocking. The first time a user enables this, they must accept a clear, strongly-worded disclaimer about the potential risks to their hardware.</item>
                        <item name="Profile Editor">Provide a UI for users to create, edit, and delete OC profiles for different algorithms.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Log Completion of Task 4.1</summary>
                <log_entry>
                     <validation_method>Conducted an integration test on a dedicated hardware rig. Verified that OC features were disabled by default. After enabling expert mode and defining profiles in the config, started the daemon in auto-profit mode. Using hardware monitoring tools (`nvidia-smi`), observed the daemon correctly apply a specific OC profile for Kaspa. After a simulated profitability switch, observed the daemon correctly revert to defaults and then apply a different, correct profile for Ravencoin. Stopping the daemon successfully reverted all hardware settings to their stock values. All validation criteria are met.</validation_method>
                     <review_outcome>Approved.</review_outcome>
                </log_entry>
                <git_commit>
                    <commands>
                        <command>git add daemon/src/overclocking.rs daemon/src/power_tuning.rs</command>
                        <command>git commit -m "Phase 4.1: Implemented Daemon Adaptive Overclocking & Power Tuning Engine."</command>
                        <command>git push origin develop</command>
                    </commands>
                    <remote_repository>https://github.com/emiliancristea/bunker-miner.git</remote_repository>
                    <branch>develop</branch>
                </git_commit>
            </action_item>
        </actions>

        <design_rationale>
            Adaptive overclocking is a powerful differentiator that directly increases user profitability. However, it is also the most dangerous feature in the application. The "security-first, safety-first" design, which makes it explicitly opt-in behind a warning and requires elevated privileges, is a non-negotiable approach to user safety and liability management. The failsafe logic to revert settings on exit is critical to ensure that a crashed daemon does not leave the user's hardware in an unstable or high-power state.
        </design_rationale>

        <operational_considerations>
            <item name="Hardware Variance">The stability of overclocks can vary dramatically between individual cards. The application cannot guarantee that a user's chosen settings will be stable. The documentation and UI must make it clear that the user is responsible for tuning and testing their own profiles.</item>
            <item name="Driver Dependency">This feature is highly dependent on the vendor's command-line tools and libraries (NVML, ROCm-SMI). An update to a GPU driver could potentially break this functionality, requiring an update to our daemon.</item>
        </operational_considerations>

        <validation_criteria>
            - The daemon successfully applies specified overclock settings to both NVIDIA and AMD GPUs.
            - The profit-switching engine correctly applies the algorithm-specific profile during a switch.
            - Stopping the daemon correctly and reliably reverts all hardware settings to default.
            - The feature is disabled by default and requires explicit user consent via the "expert mode" toggle.
            - The daemon correctly handles cases where it does not have the necessary admin/root privileges.
        </validation_criteria>
        
        <testing_methodologies>
            <item name="Physical Hardware Testing">This feature can only be properly validated on real, physical hardware. A dedicated test rig with both NVIDIA and AMD cards is required.</item>
            <item name="Manual System Testing">The primary validation method is to run the daemon and use external, trusted tools (like `nvidia-smi` or MSI Afterburner) to verify that the clock and power settings are actually being applied and reverted correctly.</item>
        </testing_methodologies>
        
        <version_control_strategy>
            <item name="Branching">The adaptive overclocking engine is developed on a `feature/daemon-adaptive-oc` branch.</item>
            <item name="Commits">The Git Commit message for this task will be exactly as specified.</item>
        </version_control_strategy>
        
        <security_audit_checkpoints>
            <checkpoint>A mandatory, exhaustive security review of the `overclock` module is required. The review must focus on the privilege handling, the failsafe mechanisms, and ensuring there are no vulnerabilities that could allow a non-admin user or a remote actor to control hardware settings.</checkpoint>
            <checkpoint>The wording of the "expert mode" disclaimer must be reviewed and approved to ensure legal and ethical clarity.</checkpoint>
            <checkpoint>The new gRPC endpoints for OC must be reviewed to ensure they are adequately protected and cannot be abused.</checkpoint>
        </security_audit_checkpoints>
    </task>
</prompt>