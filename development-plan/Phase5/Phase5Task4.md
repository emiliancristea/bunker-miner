<prompt>
    <system_prompt>
        You are the Lead Principal Engineer, with ultimate authority on security and full-stack architecture for this project. Your immediate mission is to execute Task 5.4. All ecosystem features for the MVE are now implemented. You will now conduct the final, comprehensive integration testing campaign to validate the Hashpower Marketplace and the Plugin SDK as a single, cohesive system. You will personally orchestrate the end-to-end test scenarios, including a full marketplace lifecycle and a security validation of the plugin sandbox. Upon successful validation, you will formalize the Phase 5 Deliverable, conduct the closure review, and give the final sign-off, confirming that our community-driven ecosystem is complete, robust, and secure. Your output will be the direct result of completing and self-validating these actions.
    </system_prompt>

    <task>
        <task_id>5.4</task_id>
        <task_title>Integration Testing & Phase 5 Deliverable</task_title>
        
        <technical_references>
            <reference>All Phase 5 Test Plans.</reference>
            <reference>Final BUNKER MINER ecosystem builds (daemon, client, pool, web dashboard).</reference>
            <reference>Malicious test plugin (from Task 5.3).</reference>
        </technical_references>

        <context>
            All ecosystem features for Phase 5—the Hashpower Marketplace and the Plugin SDK—have been implemented. However, their true functionality and the security of their interactions can only be validated by testing them as a complete, interconnected system. This final task validates the entire ecosystem, from the web dashboard for buyers, to the desktop client for sellers, to the secure loading of community plugins.
        </context>

        <measurable_objectives>
            <sub_objective name="Validation">
                <item>The entire BUNKER MINER ecosystem is proven to be stable and functional in the staging environment.</item>
                <item>A full end-to-end integration test of the hashpower marketplace, from order creation to seller payout, passes successfully.</item>
                <item>The WASM-based plugin system is validated to be secure and functional under real-world conditions.</item>
            </sub_objective>
            <sub_objective name="Project Management">
                <item>The Phase 5 Deliverable is formally documented and defined.</item>
                <item>A Phase 5 closure review meeting is successfully conducted, resulting in a formal "Go" decision to proceed to the final production hardening phase.</item>
            </sub_objective>
        </measurable_objectives>

        <actions>
            <action_item>
                <summary>Conduct Comprehensive End-to-End Integration Testing</summary>
                <details>
                    <sub_action name="Execute full-loop test scenarios in the staging environment">
                        <item name="Scenario 1: Full Marketplace Lifecycle">
                            <ol>
                                <li>A "buyer" user deposits funds into their account on the web dashboard.</li>
                                <li>The buyer places a large, long-running order for KawPow hashrate to their external pool address.</li>
                                <li>Two "seller" rigs, configured via the desktop client in "seller mode," are automatically assigned by the matching engine to fulfill the order.</li>
                                <li>**Verify:** The buyer's web dashboard shows the combined hashrate being delivered in real time.</li>
                                <li>After one hour, the buyer cancels the order via the web dashboard.</li>
                                <li>**Verify:** The seller rigs are commanded to stop mining for the order and return to an idle state.</li>
                                <li>**Verify:** The payout engine runs, and the sellers' accounts are correctly credited for the exact value of the shares they delivered.</li>
                            </ol>
                        </item>
                        <item name="Scenario 2: Plugin SDK Security and Functionality Validation">
                            <ol>
                                <li>Take a daemon running a hardcoded, built-in miner adapter (e.g., for lolMiner) and record its stable hashrate.</li>
                                <li>Remove the hardcoded adapter and replace it with the `lolminer.wasm` plugin.</li>
                                <li>**Verify:** The daemon successfully loads the plugin and achieves the same stable hashrate, proving functional parity.</li>
                                <li>Stop the daemon. Place a malicious test plugin (designed to read `/etc/passwd`) into the `/plugins` directory.</li>
                                <li>Start the daemon.</li>
                                <li>**Verify:** The daemon logs a security violation, indicating the plugin's attempt to access the filesystem was blocked by the sandbox, and the malicious plugin is disabled. The daemon itself remains stable.</li>
                            </ol>
                        </item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Formalize and Approve Phase 5 Deliverable</summary>
                <details>
                    <sub_action name="Document the formal Phase 5 Deliverable">
                        <item name="Deliverable Definition">"A complete, community-driven mining ecosystem. The platform includes a transparent, low-fee, peer-to-peer hashpower marketplace. A secure, sandboxed Plugin SDK empowers the community to keep the software at the cutting edge by adding support for new mining software. BUNKER MINER is now not just a tool, but a dynamic and extensible platform."</item>
                    </sub_action>
                    <sub_action name="Conduct a Phase 5 closure review meeting">
                        <item name="Agenda">
                            <ul>
                                <li>Demonstrate the successful E2E integration test scenarios for both the Marketplace and the Plugin SDK.</li>
                                <li>Review the final state of the codebase and its security posture after implementing these complex ecosystem features.</li>
                                <li>Obtain formal sign-off on the Phase 5 Deliverable from all stakeholders.</li>
                            </ul>
                        </item>
                    </sub_action>
                </details>
            </action_item>
            
            <action_item>
                <summary>Log Completion of Task 5.4</summary>
                <log_entry>
                     <validation_method>Successfully executed the full end-to-end marketplace lifecycle test, verifying correct order matching, hashpower delivery, and transactional payouts. Successfully validated the Plugin SDK's functionality and security, demonstrating both functional parity with a migrated plugin and the successful sandboxing of a malicious plugin. The Phase 5 deliverable was demonstrated and formally signed off in the closure meeting. All validation criteria are met.</validation_method>
                     <review_outcome>Phase 5 Approved for Closure.</review_outcome>
                </log_entry>
                <git_commit>
                    <commands>
                        <command>git add daemon/ client/ web/ pool/ docs/progress_logs/progress_phase_5.md</command>
                        <command>git commit -m "Phase 5.4: Full Integration Testing for Marketplace & Plugin SDK Passed; Phase 5 Complete."</command>
                        <command>git push origin develop</command>
                    </commands>
                    <remote_repository>https://github.com/emiliancristea/bunker-miner.git</remote_repository>
                    <branch>develop</branch>
                </git_commit>
            </action_item>
        </actions>

        <design_rationale>
            This final, holistic integration test is the ultimate validation gate for the ecosystem features. It confirms that the complex interplay between the marketplace, the fleet controller, our own pool's share validation, and the remote daemons is not just functional but also economically sound and secure. Validating the security of the plugin sandbox under realistic conditions is the most critical step in this task, as it gives us the confidence to open up our platform to community contributions.
        </design_rationale>

        <operational_considerations>
            <item name="Market Liquidity">For the hashpower marketplace to be successful at launch, it will need both buyers and sellers. Operational and marketing efforts will be needed to ensure there is enough liquidity on both sides of the market.</item>
            <item name="Plugin Repository">A community-facing system for submitting, reviewing, and distributing plugins (e.g., a dedicated website or GitHub organization) will be required. This is an operational task that flows from the successful completion of the SDK.</item>
        </operational_considerations>

        <validation_criteria>
            - The full end-to-end hashpower marketplace lifecycle test passes successfully.
            - A migrated plugin demonstrates functional parity with its hardcoded predecessor.
            - The plugin sandbox is proven to prevent a malicious test plugin from performing unauthorized actions.
            - The Phase 5 Deliverable is formally signed off.
        </validation_criteria>
        
        <testing_methodologies>
            <item name="End-to-End (E2E) System Testing">The primary validation is a full, scripted test of the entire platform stack, simulating complex user journeys for both buyers and sellers in the marketplace.</item>
            <item name="Security Penetration Testing">The validation of the plugin sandbox involves actively trying to break out of it, a form of focused penetration testing.</item>
        </testing_methodologies>
        
        <version_control_strategy>
            <item name="Branching">The integration tests and any necessary bug fixes are developed on a `feature/p5-integration-tests` branch.</item>
            <item name="Commits">The Git Commit message for this task will be exactly as specified.</item>
        </version_control_strategy>
        
        <security_audit_checkpoints>
            <checkpoint>The Security Lead must review and approve the results of the final E2E tests, with a particular focus on the economic integrity of the marketplace and the security of the plugin system under test.</checkpoint>
            <checkpoint>The results of the malicious plugin test must be formally documented and signed off by the Security Lead. This is the final security gate for the MVE's feature set.</checkpoint>
        </security_audit_checkpoints>
    </task>
</prompt>