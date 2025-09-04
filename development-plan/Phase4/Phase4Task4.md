<prompt>
    <system_prompt>
        You are the Lead Principal Engineer, with ultimate authority on security and full-stack architecture for this project. Your immediate mission is to execute Task 4.4. All advanced features for this phase are now implemented. You will now conduct the final, comprehensive integration testing campaign to validate the Adaptive Overclocking Engine and the Fleet Management system as a single, cohesive ecosystem. You will personally orchestrate the end-to-end test scenarios, from a remote command on the web dashboard to a hardware state change on a physical rig, including the dynamic application of per-algorithm OC profiles. Upon successful validation, you will formalize the Phase 4 Deliverable, conduct the closure review, and give the final sign-off, confirming that BUNKER MINER is a truly professional-grade, intelligent mining platform. Your output will be the direct result of completing and self-validating these actions.
    </system_prompt>

    <task>
        <task_id>4.4</task_id>
        <task_title>Integration Testing & Phase 4 Deliverable</task_title>
        
        <technical_references>
            <reference>All Phase 4 Test Plans.</reference>
            <reference>Final BUNKER MINER daemon and C++/Qt client builds.</reference>
            <reference>Final Fleet Management web application build.</reference>
            <reference>Production Monitoring Stack (Prometheus, Grafana).</reference>
        </technical_references>

        <context>
            All advanced features for Phase 4—the Adaptive OC Engine and the Fleet Management System—have been implemented. However, their true functionality and the security of their interaction can only be validated by testing them as a complete, interconnected system. This final task validates the entire ecosystem, from the web dashboard down to the hardware control on a remote machine, ensuring all components work together seamlessly and securely.
        </context>

        <measurable_objectives>
            <sub_objective name="Validation">
                <item>The entire BUNKER MINER ecosystem (daemon, client, pool, fleet controller) is proven to be stable and functional in the staging environment.</item>
                <item>A full end-to-end integration test of the Adaptive OC and Fleet Management features passes successfully.</item>
            </sub_objective>
            <sub_objective name="Project Management">
                <item>The Phase 4 Deliverable is formally documented and defined.</item>
                <item>A Phase 4 closure review meeting is successfully conducted, resulting in a formal "Go" decision to proceed to Phase 5.</item>
            </sub_objective>
        </measurable_objectives>

        <actions>
            <action_item>
                <summary>Conduct Comprehensive End-to-End Integration Testing</summary>
                <details>
                    <sub_action name="Execute full-loop test scenarios in the staging environment">
                        <item name="Scenario 1: Adaptive OC & Profit Switching">
                            <ol>
                                <li>On a test rig, use the C++/Qt client to enable "expert mode" and define specific OC profiles for Kaspa (kHeavyHash) and Ravencoin (KawPow).</li>
                                <li>Start the daemon in `start --auto` profit-switching mode.</li>
                                <li>Using a mock market data API, make Kaspa the most profitable coin.</li>
                                <li>**Verify:** Using hardware monitoring tools (`nvidia-smi`/`rocm-smi`), confirm that the specific Kaspa OC profile has been applied.</li>
                                <li>Manipulate the API to make Ravencoin more profitable.</li>
                                <li>**Verify:** The daemon correctly switches to mining Ravencoin, and the hardware monitoring tools show that it has first reverted to defaults and then applied the specific Ravencoin OC profile.</li>
                            </ol>
                        </item>
                        <item name="Scenario 2: Multi-Rig Fleet Management & Remote Control">
                            <ol>
                                <li>Set up two separate physical test rigs running the daemon in "fleet mode," linked to the same user account.</li>
                                <li>Log in to the web dashboard and verify both rigs are displayed with correct, independent telemetry.</li>
                                <li>From the web dashboard, issue a `REMOTE_STOP` command to Rig 1.</li>
                                <li>From the web dashboard, issue a `REMOTE_RESTART_MINER` command to Rig 2.</li>
                                <li>**Verify:** Observe the physical rigs (or their logs) to confirm that Rig 1 stops mining entirely, and Rig 2's miner process is terminated and restarted.</li>
                                <li>**Verify:** The web dashboard UI correctly reflects the new states of both rigs in real time.</li>
                            </ol>
                        </item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Formalize and Approve Phase 4 Deliverable</summary>
                <details>
                    <sub_action name="Document the formal Phase 4 Deliverable">
                        <item name="Deliverable Definition">"An advanced, professional-grade mining platform featuring a web-based fleet management system for multi-rig monitoring and control, and an adaptive performance engine that automatically applies per-algorithm overclocking profiles to maximize efficiency. The entire ecosystem is securely integrated and operational."</item>
                    </sub_action>
                    <sub_action name="Conduct a Phase 4 closure review meeting">
                        <item name="Agenda">
                            <ul>
                                <li>Demonstrate the successful E2E integration test scenarios for both Adaptive OC and Fleet Management.</li>
                                <li>Review the final state of the codebase and its security posture after implementing these high-risk features.</li>
                                <li>Obtain formal sign-off on the Phase 4 Deliverable from all stakeholders.</li>
                            </ul>
                        </item>
                    </sub_action>
                </details>
            </action_item>
            
            <action_item>
                <summary>Log Completion of Task 4.4</summary>
                <log_entry>
                     <validation_method>Successfully executed the full end-to-end integration test scenarios. The Adaptive OC engine correctly applied per-algorithm profiles during a profit-driven switch, as verified by external hardware monitoring tools. The Fleet Management system successfully executed remote start/stop commands on multiple physical rigs from the web dashboard. The Phase 4 deliverable was demonstrated and formally signed off. All validation criteria are met.</validation_method>
                     <review_outcome>Phase 4 Approved for Closure.</review_outcome>
                </log_entry>
                <git_commit>
                    <commands>
                        <command>git add daemon/ client/ web/ pool/ docs/progress_logs/progress_phase_4.md</command>
                        <command>git commit -m "Phase 4.4: Full Integration Testing for Adaptive OC & Fleet Management Passed; Phase 4 Complete."</command>
                        <command>git push origin develop</command>
                    </commands>
                    <remote_repository>https://github.com/emiliancristea/bunker-miner.git</remote_repository>
                    <branch>develop</branch>
                </git_commit>
            </action_item>
        </actions>

        <design_rationale>
            This final, holistic integration test is the ultimate validation gate for the advanced features. It moves beyond testing components in isolation and validates the complex, emergent behavior of the entire interconnected system, from a click on a website to a hardware state change on a remote machine. Successfully passing these scenarios provides the highest possible confidence in the stability, security, and functionality of BUNKER MINER as a professional-grade platform.
        </design_rationale>

        <operational_considerations>
            <item name="Test Environment">These tests require a complex setup: multiple physical machines, a fully deployed backend in the staging cloud environment, and the web dashboard. The maintenance of this E2E test environment is now a critical operational task.</item>
            <item name="Documentation">The successful completion of these features means user documentation is now a critical path item. Guides will be needed for setting up fleet mode and configuring OC profiles.</item>
        </operational_considerations>

        <validation_criteria>
            - The Adaptive OC engine correctly applies and reverts per-algorithm hardware profiles during an automated profit switch.
            - The Fleet Management system correctly relays commands from the web dashboard and executes them on the target remote daemons.
            - The entire ecosystem remains stable and performant during the E2E tests.
            - The Phase 4 Deliverable is formally signed off.
        </validation_criteria>
        
        <testing_methodologies>
            <item name="End-to-End (E2E) System Testing">The primary validation is a full, scripted test of the entire platform stack, simulating complex user journeys that involve all new features interacting together.</item>
            <item name="Physical Hardware Validation">For the OC engine, validation requires using external, trusted hardware monitoring tools to confirm that the software is having the intended effect on the physical hardware.</item>
        </testing_methodologies>
        
        <version_control_strategy>
            <item name="Branching">The integration tests and any necessary bug fixes are developed on a `feature/p4-integration-tests` branch.</item>
            <item name="Commits">The Git Commit message for this task will be exactly as specified.</item>
        </version_control_strategy>
        
        <security_audit_checkpoints>
            <checkpoint>The Security Lead must review the results of the final E2E tests to confirm that the remote command and control system is secure and not vulnerable to abuse under real-world conditions.</checkpoint>
            <checkpoint>The failsafe mechanisms of the OC engine must be re-validated during these tests to ensure they are robust.</checkpoint>
        </security_audit_checkpoints>
    </task>
</prompt>