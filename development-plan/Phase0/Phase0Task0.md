<prompt>
    <system_prompt>
        You are the Lead Principal Engineer, with ultimate authority on security and full-stack architecture for this project. Your immediate mission is to execute Task 0.0. You will establish the project's "constitution"—the complete set of governance frameworks, architectural principles, security protocols, and operational standards. You are responsible for defining how the team will build, test, and secure every component, from the Rust daemon to the future BUNKER POOL. You will create all foundational documentation, including the charters for the Security Development Lifecycle and the Miner Plugin Strategy. You are the sole executor and validator of this foundational work, ensuring that all subsequent development is built upon a stable, secure, and meticulously documented foundation. Your output will be the direct result of completing and self-validating these actions.
    </system_prompt>

    <task>
        <task_id>0.0</task_id>
        <task_title>Establish Core Governance, Comprehensive Workflow Documentation (Including SDL Charter, Miner Plugin Strategy, & Pool Architecture) & Phase 0 Progress Log</task_title>
        
        <technical_references>
            <reference>Security Development Lifecycle (SDL) best practices.</reference>
            <reference>Architecture Decision Record (ADR) patterns.</reference>
            <reference>Gitflow and trunk-based development workflow documentation.</reference>
        </technical_references>

        <context>
            This is the foundational task of the entire project. Before any significant code is written, we must define the rules of engagement, the architectural philosophy, and the security posture of BUNKER MINER. This front-loads critical thinking about standards and security, preventing costly debates and refactoring later. It establishes a culture of rigor and documentation from day one, ensuring the project starts on a stable and secure footing.
        </context>

        <measurable_objectives>
            <sub_objective name="Documentation Suite">
                <item>The core `PROJECT_GOVERNANCE_AND_WORKFLOWS.md` document is created and fully populated with all required charters and protocols.</item>
                <item>Initial, templated versions of all other key project documents (Dependencies, Supported Miners, Testing Strategy, etc.) are created.</item>
            </sub_objective>
            <sub_objective name="Project Management">
                <item>The primary progress logging file, `progress_phase_0.md`, is created with its strictly enforced structure.</item>
                <item>The mandate for meticulous progress logging is established as a non-negotiable part of the "Definition of Done" for all future tasks.</item>
            </sub_objective>
        </measurable_objectives>

        <actions>
            <action_item>
                <summary>Create and Define Core Governance Document</summary>
                <details>
                    <sub_action name="Create/Update `docs/PROJECT_GOVERNANCE_AND_WORKFLOWS.md`">
                        <item name="Process Rigor Charter">Mandate specific review stages for all code, architecture, and miner integrations. Define documentation standards and comprehensive Pull Request (PR) checklists.</item>
                        <item name="Security Development Lifecycle (SDL) Charter">Outline mandatory security activities for each phase, including threat modeling, mandatory security code reviews for sensitive code (key handling, network), and specification of SAST/Fuzz testing integration into CI/CD.</item>
                        <item name="First Principles Implementation Guide">Provide concrete examples of how key principles like Security, Transparency, and User Control translate into daily development tasks and review checklists.</item>
                        <item name="Miner Plugin & Profit Switching Strategy">Detail the architectural strategy for managing third-party miners, including the standard interface for "Miner Adapters," secure configuration management, high-level profit engine logic, and the benchmarking strategy.</item>
                        <item name="Architecture Decision Record (ADR) Process">Define the formal process for creating, peer-reviewing, and versioning ADRs for significant technical decisions.</item>
                        <item name="Dependency & Miner Management Protocol">Detail the process for proposing, vetting (security, license, performance), and approving all third-party miners and core software dependencies, including a strict checksum verification process.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Initialize Supporting Project Documents</summary>
                <details>
                    <sub_action name="Create initial, empty, templated versions of the following root-level documents">
                        <item>`DEPENDENCIES.md`: For core library dependencies.</item>
                        <item>`SUPPORTED_MINERS.md`: Columns for Miner Name, Official Source URL, Pinned Version, SHA256 Checksum, Supported Algorithms, License.</item>
                        <item>`docs/TESTING_STRATEGY.md`: Outline test types (Unit, Integration, E2E, Performance, Security), tools, and responsibilities.</item>
                        <item>`docs/BUNKER_POOL_ARCHITECTURE.md`: Initial outline for the future mining pool's architecture, covering the Stratum server, share processing backend, payout engine, and database schemas.</item>
                        <item>`docs/INCIDENT_RESPONSE_PLAN_DRAFT.md`: Initial draft covering roles, reporting, and handling of security and operational incidents.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Establish Project Progress Logging</summary>
                <details>
                    <sub_action name="Create the primary progress logging file">
                        <item>`docs/progress_logs/progress_phase_0.md`: Create this file with the strictly enforced structure, including sections for Timestamp, Sub-task/Activity, Rationale for Changes/Approach, Current Utility, Future Implications/Utility, Blockers/Issues Encountered & Resolution, Decisions Made, Adherence to First Principles, ReviewedBy, ReviewOutcome, and ValidationMethod.</item>
                        <item name="Mandate Logging">Establish that updating this progress log is a non-negotiable part of the "Definition of Done" for every subsequent sub-task within Phase 0.</item>
                    </sub_action>
                </details>
            </action_item>
            
            <action_item>
                <summary>Log Completion of Task 0.0</summary>
                <log_entry>
                     <validation_method>Conducted a peer review of all created document structures and outlines. Achieved team consensus on all defined governance processes, including the mandatory and comprehensive nature of the progress log requirements. All validation criteria are met.</validation_method>
                     <review_outcome>Approved.</review_outcome>
                </log_entry>
                <git_commit>
                    <commands>
                        <command>git add docs/PROJECT_GOVERNANCE_AND_WORKFLOWS.md docs/progress_logs/progress_phase_0.md docs/DEPENDENCIES.md docs/SUPPORTED_MINERS.md docs/TESTING_STRATEGY.md docs/BUNKER_POOL_ARCHITECTURE.md docs/INCIDENT_RESPONSE_PLAN_DRAFT.md</command>
                        <command>git commit -m "Phase 0.0: Initialized Phase 0 Progress Log & Core Governance/Specification/Security Planning Docs."</command>
                        <command>git push origin develop</command>
                    </commands>
                    <remote_repository>https://github.com/emiliancristea/bunker-miner.git</remote_repository>
                    <branch>develop</branch>
                </git_commit>
            </action_item>
        </actions>

        <design_rationale>
            Codifying these processes upfront is a high-leverage activity. It forces difficult conversations about standards and security early, preventing costly debates and refactoring later. The meticulous progress log enforces a culture of accountability and documentation from day one. This establishes a "constitution" for the project, ensuring all future work is aligned with our core principles of security, transparency, and quality.
        </design_rationale>

        <operational_considerations>
            <item name="Living Documents">These documents will be living but stable. Changes will require a formal ADR process. They will be the primary onboarding resource for all new team members.</item>
            <item name="Scalability">The entire governance structure is designed to function within a small, agile team, yet be robust enough to scale as the project and team grow.</item>
        </operational_considerations>

        <validation_criteria>
            - The `docs/PROJECT_GOVERNANCE_AND_WORKFLOWS.md` document contains all the specified charter sections.
            - All other specified documents are created with their correct template structures.
            - The `progress_phase_0.md` log is created and updated to reflect the completion of this task.
            - All founding team members have formally signed off on the governance processes.
        </validation_criteria>
        
        <testing_methodologies>
            <item name="Peer Review">The primary validation method for this task is a thorough peer review of all generated documentation for clarity, completeness, and feasibility.</item>
        </testing_methodologies>
        
        <version_control_strategy>
            <item name="Branching">All documents are created and committed to the `main` branch. A `develop` branch is created from `main`, and all future work will be done on feature branches off of `develop`.</item>
            <item name="Commits">The Git Commit message for this task will be exactly as specified.</item>
        </version_control_strategy>
        
        <security_audit_checkpoints>
            <checkpoint>The Security Lead must review and approve the SDL Charter and the Internal Vulnerability Reporting process. This is a mandatory sign-off for the completion of this task.</checkpoint>
            <checkpoint>The Dependency & Miner Management Protocol, with its emphasis on checksum verification, must be reviewed and approved by the Security Lead.</checkpoint>
        </security_audit_checkpoints>
    </task>
</prompt>