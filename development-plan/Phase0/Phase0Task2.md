<prompt>
    <system_prompt>
        You are the Lead Principal Engineer, with ultimate authority on security and full-stack architecture for this project. Your immediate mission is to execute Task 0.2. You will now de-risk the project by replacing all major technical assumptions with hard, empirical data. You will personally design and execute a series of focused, security-aware Proof-of-Concept (PoC) implementations for every critical and unproven technology in our stack. This includes GPU/CPU hardware interaction, third-party miner process management, the client-daemon IPC mechanism, and the core components of our future Stratum pool server. You will not commit to any technology until you have personally validated its performance, stability, and security posture. Your output will be the direct result of completing and self-validating these actions.
    </system_prompt>

    <task>
        <task_id>0.2</task_id>
        <task_title>Technology Choices & Core Libraries Finalization (with Security-Focused PoC Validation)</task_title>
        
        <technical_references>
            <reference>NVIDIA Management Library (NVML) SDK Documentation.</reference>
            <reference>AMD ROCm-SMI or ADL SDK Documentation.</reference>
            <reference>gRPC and Protocol Buffers Documentation.</reference>
            <reference>Official documentation for all evaluated libraries (e.g., `tonic`, `age`, `tokio`).</reference>
            <reference>docs/PROJECT_GOVERNANCE_AND_WORKFLOWS.md (specifically the SDL Charter).</reference>
        </technical_references>

        <context>
            Our architecture, defined in Task 0.1, relies on a complex stack of interoperating technologies. Foundational choices, such as the specific libraries for GPU interaction, the client-daemon IPC mechanism, and secure storage, carry immense architectural weight. Making these commitments based on documentation alone is a recipe for disaster. This task mandates a rigorous, empirical validation phase where we build small, focused Proof-of-Concept implementations to de-risk the project by replacing assumptions with hard data on performance, security, and integration feasibility.
        </context>

        <measurable_objectives>
            <sub_objective name="Technology Validation">
                <item>A final, binding decision on all core libraries (GPU/CPU detection, process management, IPC, secure storage) is made and justified in new ADRs.</item>
                <item>A signed-off PoC report, including a detailed security assessment and performance benchmarks, is produced for every critical technology listed.</item>
            </sub_objective>
            <sub_objective name="Documentation">
                <item>The `DEPENDENCIES.md` and `SUPPORTED_MINERS.md` documents are populated with the final, validated library choices and their exact pinned versions.</item>
            </sub_objective>
        </measurable_objectives>

        <actions>
            <action_item>
                <summary>Execute Proof-of-Concept Implementations for Critical Technologies</summary>
                <details>
                    <sub_action name="GPU/CPU Detection PoC (Rust)">
                        <item name="Objective">Validate the ability to reliably detect and query all necessary hardware information on both Windows and Linux.</item>
                        <item name="Tasks">
                            <ul>
                                <li>Implement a test application using `nvml-wrapper` to connect to the NVIDIA driver and query GPU name, temperature, power usage, and core/memory clocks.</li>
                                <li>Implement a similar test application for AMD using a `rocm-smi` CLI wrapper or the `adl` crate.</li>
                                <li>Verify that `sysinfo` or `raw-cpuid` can correctly identify the CPU model.</li>
                            </ul>
                        </item>
                        <item name="Success Criteria">The PoC application correctly identifies all test hardware and provides stable, accurate telemetry readings without crashing.</item>
                    </sub_action>
                    <sub_action name="Miner Process Management PoC (Rust)">
                        <item name="Objective">Validate the ability to robustly control and monitor a third-party miner as a child process.</item>
                        <item name="Tasks">
                            <ul>
                                <li>Build a test harness that uses `tokio::process::Command` to start and stop a real miner (e.g., XMRig).</li>
                                <li>Implement a real-time stdout/stderr parser that correctly extracts hashrate and share acceptance messages.</li>
                                <li>Test the ability to forcefully terminate (`kill`) the process and handle a simulated crash.</li>
                            </ul>
                        </item>
                        <item name="Success Criteria">The harness can reliably start, stop, and monitor the miner. It correctly detects unexpected process termination.</item>
                    </sub_action>
                    <sub_action name="Client-Daemon IPC PoC (Rust & C++/Qt)">
                        <item name="Objective">Validate gRPC as the definitive, high-performance, and type-safe communication layer between the Rust daemon and the C++ client.</item>
                        <item name="Tasks">
                            <ul>
                                <li>Define a simple `.proto` file with a `Ping/Pong` service.</li>
                                <li>Implement a minimal gRPC server in the Rust daemon PoC using `tonic`.</li>
                                <li>Implement a minimal gRPC client in the C++ client PoC using the official gRPC C++ library.</li>
                                <li>Benchmark request/response latency and test streaming capabilities.</li>
                            </ul>
                        </item>
                        <item name="Success Criteria">The C++ client can successfully make RPC calls to the Rust server. A new ADR is created, justifying the choice of gRPC.</item>
                    </sub_action>
                    <sub_action name="Stratum Pool Server PoC (Rust)">
                        <item name="Objective">De-risk future BUNKER POOL development by validating the core components of a Stratum server.</item>
                        <item name="Tasks">
                            <ul>
                                <li>Build a minimal Stratum v1 TCP server using `tokio` that listens on a port.</li>
                                <li>Implement the basic Stratum handshake: it must accept a `mining.subscribe` message from a real miner and respond with a valid session ID.</li>
                                <li>Implement the ability to send a dummy `mining.notify` (job) message to the connected miner.</li>
                            </ul>
                        </item>
                        <item name="Success Criteria">A real third-party miner (e.g., lolMiner) can successfully connect to the PoC server and stay connected, receiving jobs.</item>
                    </sub_action>
                    <sub_action name="Secure Storage PoC (Rust)">
                        <item name="Objective">Validate a secure and user-friendly method for encrypting local configuration files.</item>
                        <item name="Tasks">
                            <ul>
                                <li>Use the `age` crate to implement a function that encrypts a sample `config.toml` file with a user-provided password.</li>
                                <li>Implement a corresponding function that decrypts the file.</li>
                            </ul>
                        </item>
                        <item name="Success Criteria">The file is successfully encrypted and decrypted. The process is robust and handles incorrect passwords gracefully.</item>
                    </sub_action>
                </details>
            </action_item>
            
            <action_item>
                <summary>Conduct PoC Showcases and Finalize Documentation</summary>
                <details>
                    <sub_action name="Showcase and Review">
                        <item>Conduct an internal showcase for each completed PoC to all relevant leads.</item>
                        <item>For each PoC, create a report in the progress log that includes a mandatory "Security Assessment & Hardening" section, documenting potential vulnerabilities and proposed mitigations.</item>
                    </sub_action>
                    <sub_action name="Update Documentation">
                        <item>Update or create ADRs with the finalized technology choices, detailed security assessments, and justifications based on PoC results.</item>
                        <item>Populate `DEPENDENCIES.md` and `SUPPORTED_MINERS.md` with all chosen libraries, tools, and their pinned versions.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Log Completion of Task 0.2</summary>
                <log_entry>
                     <validation_method>Successfully demonstrated each PoC to the relevant technical leads. All PoC reports, including their mandatory Security Assessments, have been peer-reviewed and signed off. All technology choices have been formally approved and recorded in the respective ADRs. The `DEPENDENCIES.md` and `SUPPORTED_MINERS.md` documents are now populated. All validation criteria are met.</validation_method>
                     <review_outcome>Approved.</review_outcome>
                </log_entry>
                <git_commit>
                    <commands>
                        <command>git add docs/DEPENDENCIES.md docs/SUPPORTED_MINERS.md docs/ADRs/</command>
                        <command>git commit -m "Phase 0.2: Technology PoCs & Security Assessments Completed; Finalized Tech Stack & Dependencies."</command>
                        <command>git push origin develop</command>
                    </commands>
                    <remote_repository>https://github.com/emiliancristea/bunker-miner.git</remote_repository>
                    <branch>develop</branch>
                </git_commit>
            </action_item>
        </actions>

        <design_rationale>
            This PoC phase is the most important risk-reduction activity in the entire plan. It replaces assumptions with hard data, allowing us to build on a foundation of proven technology rather than hope. Choosing the correct libraries for hardware interaction and IPC is critical for the stability and performance of the final product. Front-loading these validations is exponentially cheaper than discovering a fundamental incompatibility late in the development cycle.
        </design_rationale>

        <operational_considerations>
            <item name="Performance Baselines">The performance benchmarks from these PoCs will serve as the initial baseline for our production monitoring dashboards and for detecting future performance regressions.</item>
            <item name="Dependency Stability">The pinned library versions from this phase will be locked and only updated through a formal, security-vetted process, ensuring a stable and reproducible build environment.</item>
        </operational_considerations>

        <validation_criteria>
            - Each PoC is successfully demonstrated to the relevant leads.
            - A signed-off PoC Report document exists for each technology, including a detailed Security Assessment.
            - Formal approval of all technology choices is recorded in new or updated ADRs.
            - The `DEPENDENCIES.md` and `SUPPORTED_MINERS.md` documents are populated with the finalized, pinned versions.
        </validation_criteria>
        
        <testing_methodologies>
            <item name="Prototyping">Each PoC is a small, focused prototype designed to test a specific technical hypothesis.</item>
            <item name="Benchmarking">Performance-critical PoCs (like the IPC mechanism) will include quantitative benchmarks to compare alternatives.</item>
        </testing_methodologies>
        
        <version_control_strategy>
            <item name="Branching">Each PoC will be developed on a dedicated `feature/poc-[technology]` branch. Upon completion and approval, the learnings and finalized libraries are merged into `develop` via documentation updates and configuration changes.</item>
            <item name="Commits">The Git Commit message for this task will be exactly as specified.</item>
        </version_control_strategy>
        
        <security_audit_checkpoints>
            <checkpoint>The Security Lead must review and sign off on the security assessment for every PoC report. This is a non-negotiable step.</checkpoint>
            <checkpoint>The security of the chosen IPC mechanism (e.g., ensuring it can be secured with TLS) is a critical checkpoint.</checkpoint>
            <checkpoint>The choice of a secure storage library for user configuration must be approved by the Security Lead.</checkpoint>
        </security_audit_checkpoints>
    </task>
</prompt>