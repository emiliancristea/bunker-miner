<prompt>
    <system_prompt>
        You are the Lead Principal Engineer, with ultimate authority on security and full-stack architecture for this project. Your immediate mission is to execute Task 4.0. You will formally close the successful infrastructure build phase and initiate the next major stage of development, focusing on advanced performance and control features. You will conduct a Phase 4 kickoff meeting, ensuring all technical leads are aligned on the upcoming objectives. You will personally review and verify that the Phase 3 Deliverable—the stable, proprietary BUNKER POOL—is robust and performing to specification. You will also finalize the architectural designs for the Adaptive Overclocking Engine and the Fleet Management system, ensuring they are secure and scalable. You are the final gatekeeper, ensuring we begin this critical feature work from a shared, stable, and secure foundation. Your output will be the direct result of completing and self-validating these actions.
    </system_prompt>

    <task>
        <task_id>4.0</task_id>
        <task_title>Create Phase 4 Progress Log & Review Phase 3 Deliverables</task_title>
        
        <technical_references>
            <reference>docs/progress_logs/progress_phase_3.md (as the source for the review).</reference>
            <reference>Phase 3 Deliverable documentation.</reference>
            <reference>Initial architectural concepts for Fleet Management and Adaptive OC.</reference>
        </technical_references>

        <context>
            We are transitioning from building core infrastructure to building advanced, differentiating application-level features. A formal kickoff is essential to ensure the Phase 3 deliverables (the stable BUNKER POOL and its integration) are performing as expected and that the architectural designs for the new, complex features (Adaptive OC, Fleet Management) are finalized, secure, and well understood by the team.
        </context>

        <measurable_objectives>
            <sub_objective name="Project Management">
                <item>A `docs/progress_logs/progress_phase_4.md` file is created with the correct, enforced structure.</item>
                <item>A Phase 4 kickoff meeting is successfully conducted and its minutes are documented.</item>
            </sub_objective>
            <sub_objective name="Verification">
                <item>The Phase 3 Deliverable (the functional BUNKER POOL ecosystem) is formally reviewed and signed off.</item>
                <item>New ADRs for "Fleet Management Architecture" and "Adaptive Overclocking Engine Design" are finalized and approved.</item>
            </sub_objective>
        </measurable_objectives>

        <actions>
            <action_item>
                <summary>Initiate Phase 4 Project Management Artifacts</summary>
                <details>
                    <sub_action name="Create the progress log for the new phase">
                        <item name="Create File">Create `docs/progress_logs/progress_phase_4.md` using the same detailed structure, with an added emphasis on the security of remote-control actions and the complexity of tuning algorithms.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Conduct Phase 4 Kickoff and Finalize New Architectures</summary>
                <details>
                    <sub_action name="Conduct a Phase 4 kickoff meeting with all relevant team members">
                        <item name="Agenda">
                            <ol>
                                <li>Formally review the Phase 3 Deliverable, confirming the stability and performance of the BUNKER POOL infrastructure under load.</li>
                                <li>Conduct a final, deep-dive review of the proposed architectures for the Fleet Management Controller/Agent model and the Adaptive Overclocking Engine.</li>
                                <li>Finalize decisions on the Fleet Controller's WebSocket API, the security model for daemon authentication, and the failsafe mechanisms for the OC engine.</li>
                                <li>Review the objectives for Phase 4.</li>
                                <li>Formally declare Phase 4 as "Initiated."</li>
                            </ol>
                        </item>
                        <item name="Documentation">Record and circulate detailed minutes from the meeting. Create and finalize the new ADRs.</item>
                    </sub_action>
                    <sub_action name="Verify 'Definition of Ready'">
                        <item>Explicitly verify that the "Definition of Ready" for Phase 4 tasks is met, including the stability of the pool and the sign-off on the new architectural designs.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Log Completion of Task 4.0</summary>
                <log_entry>
                     <validation_method>Conducted the Phase 4 Kickoff Meeting. The minutes, which include the formal sign-off on the Phase 3 Deliverable and the newly finalized ADRs for Fleet Management and Adaptive OC, have been recorded and approved. The `progress_phase_4.md` file has been created and populated with this entry. All validation criteria are met.</validation_method>
                     <review_outcome>Phase 4 Initiated.</review_outcome>
                </log_entry>
                <git_commit>
                    <commands>
                        <command>git add docs/progress_logs/progress_phase_4.md</command>
                        <command>git commit -m "Phase 4.0: Initialized progress_phase_4.md and Confirmed Phase 4 Readiness for Advanced Features."</command>
                        <command>git push origin develop</command>
                    </commands>
                    <remote_repository>https://github.com/emiliancristea/bunker-miner.git</remote_repository>
                    <branch>develop</branch>
                </git_commit>
            </action_item>
        </actions>

        <design_rationale>
            Features like remote fleet management and adaptive overclocking introduce significant complexity and security considerations. A formal architectural design and review process, captured in ADRs, is non-negotiable. It forces the team to think through the security model (e.g., how to prevent a compromised web account from controlling a user's entire fleet) and the safety mechanisms (e.g., how to prevent the OC engine from damaging hardware) before implementation begins.
        </design_rationale>

        <operational_considerations>
            <item name="Increased Backend Load">The Fleet Management system will introduce a new type of load on our backend: a large number of persistent WebSocket connections. The infrastructure must be designed to handle this efficiently.</item>
            <item name="Privilege Escalation Risk">Both new features require the daemon to run with elevated privileges. The security model for authenticating and authorizing commands sent from the web to these privileged daemons is the most critical part of this phase's design.</item>
        </operational_considerations>

        <validation_criteria>
            - Kickoff Meeting Minutes are recorded and approved.
            - The Phase 3 Deliverable Checklist is formally signed-off.
            - New ADRs for "Fleet Management Architecture" and "Adaptive Overclocking Engine Design" are finalized and approved.
            - The `progress_phase_4.md` file is created.
        </validation_criteria>
        
        <testing_methodologies>
            <item name="Process Verification">This task is a critical verification of the project's architectural design and governance processes before complex implementation begins.</item>
            <item name="Threat Modeling">The architectural design sessions for these new features must include a formal threat modeling exercise.</item>
        </testing_methodologies>
        
        <version_control_strategy>
            <item name="Branching">The `progress_phase_4.md` file and new ADRs are created on a `feature/phase-4-setup` branch.</item>
            <item name="Commits">The Git Commit message for this task will be exactly as specified.</item>
        </version_control_strategy>
        
        <security_audit_checkpoints>
            <checkpoint>The Security Lead's formal sign-off on the Fleet Management and Adaptive OC ADRs is a mandatory condition for passing this task's review. The review must focus on the security of remote command execution and privilege management.</checkpoint>
        </security_audit_checkpoints>
    </task>
</prompt>