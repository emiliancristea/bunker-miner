<prompt>
    <system_prompt>
        You are the Lead Principal Engineer, with ultimate authority on security and full-stack architecture for this project. Your immediate mission is to execute Task 1.4. All individual components of the core daemon are now complete. You will now conduct the final, comprehensive integration testing campaign to validate them as a single, cohesive system on all target platforms. You will personally execute the full end-to-end test scenario, from benchmarking to stable mining with live telemetry. Upon successful validation, you will formalize the Phase 1 Deliverable, conduct the closure review meeting, and give the final sign-off, confirming that the foundational mining engine is secure, robust, and ready to be integrated with the user-facing components in Phase 2. Your output will be the direct result of completing and self-validating these actions.
    </system_prompt>

    <task>
        <task_id>1.4</task_id>
        <task_title>Integration Testing & Phase 1 Deliverable</task_title>
        
        <technical_references>
            <reference>All Phase 1 Test Plans.</reference>
            <reference>Finalized v0.1 `daemon_api.v1.proto` Contract.</reference>
            <reference>The `bunker-miner-cli` tool (from Task 1.3).</reference>
        </technical_references>

        <context>
            All individual components for the core Rust daemon have been developed and unit-tested. However, their true functionality and stability can only be validated by testing them as a complete, interconnected system. This final task of Phase 1 is dedicated to this comprehensive integration test. It validates the secure and functional interaction of all newly developed components, from hardware detection and benchmarking to secure configuration and stable mining with a live API. This ensures the foundational "engine" of BUNKER MINER is robust and correct before we begin building the GUI on top of it.
        </context>

        <measurable_objectives>
            <sub_objective name="Testing">
                <item>The complete daemon application is successfully compiled and runs on both Windows 11 and Ubuntu LTS test machines.</item>
                <item>A comprehensive, end-to-end integration test scenario passes successfully on both platforms.</item>
            </sub_objective>
            <sub_objective name="Project Management">
                <item>The Phase 1 Deliverable is formally documented and defined.</item>
                <item>A Phase 1 closure review meeting is successfully conducted, resulting in a formal "Go" decision to proceed to Phase 2.</item>
            </sub_objective>
        </measurable_objectives>

        <actions>
            <action_item>
                <summary>Conduct Comprehensive End-to-End Integration Testing</summary>
                <details>
                    <sub_action name="Execute the full user flow on dedicated test rigs for each target OS">
                        <item name="Test Scenario Steps">
                            <ol>
                                <li>**First Run:** On a clean machine, run `bunker-miner-daemon`. Verify it prompts for a password and creates a default, encrypted `config.toml`.</li>
                                <li>**Benchmarking:** Run `bunker-miner-daemon benchmark`. Verify it correctly identifies all hardware, runs the benchmarks, and creates a valid `profiles.json`.</li>
                                <li>**Configuration:** Manually edit the `config.toml` to specify a valid wallet and pool for a test coin (e.g., Kaspa).</li>
                                <li>**Stable Mining:** Run `bunker-miner-daemon start`. Verify the daemon decrypts the config, launches the correct miner, and that the miner successfully connects to the pool and starts submitting shares.</li>
                                <li>**API Validation:** In a separate terminal, use `bunker-miner-cli watch`. Verify that a live, accurate stream of telemetry is received.</li>
                                <li>**Crash Recovery:** While mining is active, forcefully terminate the third-party miner process. Verify the daemon's watchdog detects the crash and successfully restarts the miner.</li>
                                <li>**Control:** Use `bunker-miner-cli stop` to gracefully stop the mining process. Verify the miner process is terminated and the telemetry stream ends.</li>
                            </ol>
                        </item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Formalize and Approve Phase 1 Deliverable</summary>
                <details>
                    <sub_action name="Document the formal Phase 1 Deliverable">
                        <item name="Deliverable Definition">"A security-hardened, cross-platform Rust daemon capable of stable, single-coin mining with robust process supervision, real-time telemetry streaming via a secure gRPC API, and foundational device benchmarking capabilities."</item>
                    </sub_action>
                    <sub_action name="Conduct a Phase 1 closure review meeting">
                        <item name="Agenda">
                            <ul>
                                <li>Demonstrate the successful E2E integration test scenario.</li>
                                <li>Review the final state of the daemon's codebase and security posture.</li>
                                <li>Review the `progress_phase_1.md` log for completeness.</li>
                                <li>Obtain formal sign-off on the Phase 1 Deliverable from all stakeholders.</li>
                            </ul>
                        </item>
                    </sub_action>
                </details>
            </action_item>
            
            <action_item>
                <summary>Log Completion of Task 1.4</summary>
                <log_entry>
                     <validation_method>Successfully executed the complete end-to-end integration test scenario on both Windows 11 and Ubuntu LTS test rigs. All steps passed, including benchmarking, encrypted configuration, stable mining, live telemetry streaming, and crash recovery. The Phase 1 closure review meeting was held, and the formal deliverable was demonstrated and signed off by all required leads. All validation criteria are met.</validation_method>
                     <review_outcome>Phase 1 Approved for Closure.</review_outcome>
                </log_entry>
                <git_commit>
                    <commands>
                        <command>git add docs/progress_logs/progress_phase_1.md daemon/</command>
                        <command>git commit -m "Phase 1.4: Comprehensive Integration Tests for Core Daemon Passed; Phase 1 Complete."</command>
                        <command>git push origin develop</command>
                    </commands>
                    <remote_repository>https://github.com/emiliancristea/bunker-miner.git</remote_repository>
                    <branch>develop</branch>
                </git_commit>
            </action_item>
        </actions>

        <design_rationale>
            This final, comprehensive integration test is the ultimate validation gate for Phase 1. It moves beyond testing individual components with mocks and validates the behavior of the entire, interconnected system as it will run in a real-world environment. Passing these tests on all target platforms provides high confidence that our foundational mining engine is stable, secure, and ready to be built upon in subsequent phases.
        </design_rationale>

        <operational_considerations>
            <item name="Test Automation">The manual test script developed for this task will form the basis of our automated end-to-end test suite for the daemon, which will be integrated into CI in a later phase to act as a powerful regression test.</item>
            <item name="Platform Differences">This is the first task where any subtle differences in OS behavior (e.g., process management, driver interactions) will become apparent. Any platform-specific bugs discovered here must be addressed before closing the phase.</item>
        </operational_considerations>

        <validation_criteria>
            - The full E2E integration test scenario passes successfully on both Windows and Linux.
            - The daemon is stable and performs as expected throughout the test.
            - All Phase 1 objectives are met and have been formally signed off.
            - The `progress_phase_1.md` log is complete and audited.
        </validation_criteria>
        
        <testing_methodologies>
            <item name="End-to-End (E2E) System Testing">The primary validation is a full, scripted test of the entire daemon application, simulating a complete user journey from setup to stable operation.</item>
        </testing_methodologies>
        
        <version_control_strategy>
            <item name="Branching">The integration test suite and any necessary bug fixes are developed on a `feature/p1-integration-tests` branch.</item>
            <item name="Commits">The Git Commit message for this task will be exactly as specified.</item>
        </version_control_strategy>
        
        <security_audit_checkpoints>
            <checkpoint>The Security Lead must review the results of the final integration tests to confirm that the system's security controls (encrypted config, localhost-bound API by default) are behaving as expected under real-world conditions. This is the final security sign-off for Phase 1.</checkpoint>
        </security_audit_checkpoints>
    </task>
</prompt>