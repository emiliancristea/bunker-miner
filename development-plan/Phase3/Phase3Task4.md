<prompt>
    <system_prompt>
        You are the Lead Principal Engineer, with ultimate authority on security and full-stack architecture for this project. Your immediate mission is to execute Task 3.4. All components of the BUNKER POOL are now complete. You will now conduct the final, comprehensive integration and stress testing campaign to validate the entire client-to-pool ecosystem. You will personally orchestrate a long-duration stress test of the pool with a fleet of daemons. You will then oversee the final integration of the pool into the BUNKER MINER daemon and client, making our own infrastructure the default, preferred choice. Upon successful validation, you will formalize the Phase 3 Deliverable, conduct the closure review, and give the final sign-off, confirming that our proprietary, vertically integrated mining ecosystem is complete and robust. Your output will be the direct result of completing and self-validating these actions.
    </system_prompt>

    <task>
        <task_id>3.4</task_id>
        <task_title>Integration Testing, Daemon Update & Phase 3 Deliverable</task_title>
        
        <technical_references>
            <reference>All Phase 3 Test Plans.</reference>
            <reference>Final BUNKER MINER daemon and C++/Qt client builds (from Phase 2).</reference>
            <reference>Production Monitoring Stack (Prometheus, Grafana).</reference>
        </technical_references>

        <context>
            All individual components of the BUNKER POOL are now implemented. However, their true stability, performance, and security can only be validated by testing them as a complete, interconnected system under a realistic, sustained load. This task is the final integration test for the entire pool stack. It also involves updating our client application to make our new pool the premier, default choice, thus completing the vertical integration loop.
        </context>

        <measurable_objectives>
            <sub_objective name="Validation">
                <item>The entire BUNKER POOL stack is successfully deployed and proven to be stable under a 24-hour sustained load test in the staging environment.</item>
                <item>A full end-to-end integration test from a BUNKER MINER daemon connecting, mining, finding a block, and receiving a payout passes successfully.</item>
            </sub_objective>
            <sub_objective name="Integration & Project Management">
                <item>The BUNKER MINER daemon and C++/Qt client are updated to natively support and prioritize mining to BUNKER POOL.</item>
                <item>The Phase 3 Deliverable is formally documented and signed off by all stakeholders.</item>
            </sub_objective>
        </measurable_objectives>

        <actions>
            <action_item>
                <summary>Conduct Comprehensive Integration and Stress Testing</summary>
                <details>
                    <sub_action name="Execute a 24-hour sustained load test in the staging cloud environment">
                        <item name="Test Fleet">Deploy a fleet of at least 10 (or more, if budget allows) instances of the BUNKER MINER daemon (from Phase 2) on cloud VMs, all configured to mine to the new staging BUNKER POOL.</item>
                        <item name="Sustained Load">Let the fleet mine continuously for a 24-hour period.</item>
                        <item name="Monitoring">Throughout the test, actively monitor the production-grade Grafana dashboards. Track Stratum server connection counts, share processing latency, database query performance, and CPU/memory utilization of all pool components.</item>
                        <item name="Validation">
                            <ul>
                                <li>Verify there are no crashes, memory leaks, or performance degradation over the 24-hour period.</li>
                                <li>Manually trigger a simulated block find and verify the payout engine correctly processes the rewards for the entire test fleet.</li>
                            </ul>
                        </item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Integrate BUNKER POOL into Client Application</summary>
                <details>
                    <sub_action name="Update the BUNKER MINER daemon">
                        <item name="Default Configuration">Add BUNKER POOL as a pre-configured, default pool option in the daemon's `config.toml` template.</item>
                        <item name="Profit Engine Prioritization">Enhance the profit-switching engine to query the new BUNKER POOL API for profitability data. It will be hardcoded to trust and prioritize our own pool's data, and may even apply a lower effective fee for our own miners to incentivize usage.</item>
                    </sub_action>
                    <sub_action name="Update the C++/Qt client">
                        <item name="UI Integration">Add a new "Pool Stats" page to the client that is designed to display the rich data from the new BUNKER POOL API.</item>
                        <item name="Easy Setup">In the configuration UI, BUNKER POOL will be the default, recommended choice, making it a one-click setup for new users.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Formalize and Approve Phase 3 Deliverable</summary>
                <details>
                    <sub_action name="Document the formal Phase 3 Deliverable">
                        <item name="Deliverable Definition">"A production-ready, secure, and scalable proprietary multi-algorithm mining pool (BUNKER POOL). The pool is fully integrated with the BUNKER MINER daemon, which now defaults to and prioritizes our own infrastructure, creating a complete, vertically integrated mining ecosystem."</item>
                    </sub_action>
                    <sub_action name="Conduct a Phase 3 closure review meeting">
                        <item name="Agenda">
                            <ul>
                                <li>Present the results of the 24-hour stress test, including performance metrics and stability validation.</li>
                                <li>Demonstrate the updated client, showing the seamless integration with the new pool.</li>
                                <li>Obtain formal sign-off on the Phase 3 Deliverable from all stakeholders.</li>
                            </ul>
                        </item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Log Completion of Task 3.4</summary>
                <log_entry>
                     <validation_method>Successfully completed the 24-hour sustained load test. The BUNKER POOL stack remained stable with no performance degradation. Payouts were processed correctly. Demonstrated the updated BUNKER MINER client, which now seamlessly defaults to and displays stats from our own pool. The Phase 3 deliverable was formally signed off by all required leads in the closure meeting. All validation criteria are met.</validation_method>
                     <review_outcome>Phase 3 Approved for Closure.</review_outcome>
                </log_entry>
                <git_commit>
                    <commands>
                        <command>git add pool/ client/src/ docs/progress_logs/progress_phase_3.md</command>
                        <command>git commit -m "Phase 3.4: Full BUNKER POOL Integration & Stress Testing Passed; Phase 3 Complete."</command>
                        <command>git push origin develop</command>
                    </commands>
                    <remote_repository>https://github.com/emiliancristea/bunker-miner.git</remote_repository>
                    <branch>develop</branch>
                </git_commit>
            </action_item>
        </actions>

        <design_rationale>
            A long-duration stress test is the only way to uncover subtle, time-dependent issues like memory leaks, database connection pool exhaustion, or performance degradation under load. It provides the highest level of confidence in the stability of our new infrastructure. Tightly integrating the pool into the client as the default, premier option is a critical business strategy. It creates a powerful flywheel, where the client drives hashrate to our pool, and the pool's success allows us to offer better features and lower fees to our client users.
        </design_rationale>

        <operational_considerations>
            <item name="Staging Environment Costs">The 24-hour stress test will incur cloud computing costs. This is a planned and necessary expense for validating production readiness.</item>
            <item name="Default Experience">By making our own pool the default, we take on the responsibility for providing a reliable and profitable experience for our users from their very first run. The pool's uptime and performance are now core to the product's reputation.</item>
        </operational_considerations>

        <validation_criteria>
            - The 24-hour stress test completes successfully with no data loss, crashes, or significant performance degradation.
            - The updated BUNKER MINER daemon correctly mines to the new pool by default and prioritizes it in profit-switching.
            - The updated C++/Qt client correctly displays statistics from the new BUNKER POOL API.
            - The Phase 3 Deliverable is formally signed off.
        </validation_criteria>
        
        <testing_methodologies>
            <item name="Stress Testing">A long-duration, high-load test designed to push the system to its limits and measure its stability over time.</item>
            <item name="End-to-End (E2E) System Testing">Validating the full, updated loop from the client application, through the daemon, to the pool, and back to the client's new UI.</item>
        </testing_methodologies>
        
        <version_control_strategy>
            <item name="Branching">The final integration work and client updates are developed on a `feature/p3-integration` branch.</item>
            <item name="Commits">The Git Commit message for this task will be exactly as specified.</item>
        </version_control_strategy>
        
        <security_audit_checkpoints>
            <checkpoint>The Security Lead must review the results of the stress test for any security-related anomalies (e.g., unexpected error rates, potential DoS vectors that emerge under load).</checkpoint>
            <checkpoint>The security of the updated client-to-pool API communication must be verified to ensure it's using TLS and handles authentication correctly.</checkpoint>
        </security_audit_checkpoints>
    </task>
</prompt>