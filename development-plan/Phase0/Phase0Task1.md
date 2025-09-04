<prompt>
    <system_prompt>
        You are the Lead Principal Engineer, with ultimate authority on security and full-stack architecture for this project. Your immediate mission is to execute Task 0.1. Having established the project's governance, you will now construct the digital skeleton of BUNKER MINER. You will personally initialize the Git monorepo, define the complete, canonical directory structure for all components, and establish the version control policies and automated quality gates that will protect the codebase. Furthermore, you will create the comprehensive, security-first onboarding documentation and training program that every developer must follow, ensuring a clean, stable, and secure foundation before any significant code is written. You are the sole executor and validator of this foundational setup. Your output will be the direct result of completing and self-validating these actions.
    </system_prompt>

    <task>
        <task_id>0.1</task_id>
        <task_title>Project Setup, Version Control & Developer Onboarding</task_title>
        
        <technical_references>
            <reference>Git Documentation.</reference>
            <reference>Cargo and CMake build system specifications.</reference>
            <reference>Pre-commit hooks documentation.</reference>
            <reference>docs/PROJECT_GOVERNANCE_AND_WORKFLOWS.md (as created in Task 0.0).</reference>
        </technical_references>

        <context>
            With our core governance framework established, we must now create the physical digital workspace (the monorepo) and the standardized environment for our developers. An unstructured repository and inconsistent development environments are a primary source of bugs, security risks, and development friction. This task sets up the entire project's directory structure, version control policies with automated quality gates, and the comprehensive, security-first onboarding process that every developer must follow.
        </context>

        <measurable_objectives>
            <sub_objective name="Repository">
                <item>A functional Git monorepo with the complete, approved directory structure is initialized and pushed to the remote repository.</item>
                <item>Automated pre-commit hooks for code formatting and linting are implemented and functional.</item>
            </sub_objective>
            <sub_objective name="Onboarding">
                <item>A new developer can successfully clone the repository, follow the `DEVELOPMENT_ENVIRONMENT.md` guide, and build all initial stubs.</item>
                <item>The entire founding team has completed the initial security awareness and secure coding training session.</item>
            </sub_objective>
        </measurable_objectives>

        <actions>
            <action_item>
                <summary>Initialize Git Monorepo and Define Directory Structure</summary>
                <details>
                    <sub_action name="Initialize and configure the Git monorepo at the project root">
                        <item name="Initialize Git">Run `git init` and create the initial remote repository.</item>
                        <item name="Define Top-Level Directory Structure">
                            <ul>
                                <li>`/daemon`: For the core Rust mining agent/daemon.</li>
                                <li>`/client`: For the C++ Qt UI application.</li>
                                <li>`/pool`: For the future Rust-based BUNKER POOL server.</li>
                                <li>`/libs`: For shared libraries (e.g., `common-rust` for shared types).</li>
                                <li>`/protos`: For all gRPC API contract definitions.</li>
                                <li>`/tools`: For development and utility scripts (e.g., the `bunker-miner-cli` test harness).</li>
                                <li>`/infra`: For Infrastructure-as-Code (Terraform, Dockerfiles, Kubernetes manifests).</li>
                                <li>`/docs`: For all project documentation.</li>
                            </ul>
                        </item>
                        <item name="Initialize Project Skeletons">
                            <ul>
                                <li>Run `cargo init` in `/daemon`, `/pool`, and any sub-directories in `/libs`.</li>
                                <li>Create initial `CMakeLists.txt` shells in `/client`.</li>
                            </ul>
                        </item>
                        <item name="Configure `.gitignore`">Add a comprehensive `.gitignore` file configured for Rust, C++, Qt, and common OS files.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Implement Version Control Policies and Quality Gates</summary>
                <details>
                    <sub_action name="Implement automated pre-commit Git hooks">
                        <item name="Code Formatters">Hooks must run `rustfmt --check` for Rust and `clang-format --check` for C++.</item>
                        <item name="Linters">Hooks must run `cargo clippy -- -D warnings` for Rust.</item>
                        <item name="Enforcement">These hooks must prevent commits with formatting or linting errors.</item>
                    </sub_action>
                    <sub_action name="Define and document the branching strategy">
                        <item>Adopt and document Gitflow (main, develop, feature/, etc.).</item>
                        <item>The `main` and `develop` branches must be protected in the remote repository, requiring Pull Requests and passing CI checks before merging.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Create Comprehensive Developer Onboarding Documentation & Security Training</summary>
                <details>
                    <sub_action name="Create/Update `docs/DEVELOPMENT_ENVIRONMENT.md`">
                        <item name="Setup Instructions">Provide detailed setup instructions for target development OSes (Windows 11, Ubuntu LTS).</item>
                        <item name="Pinned Versions">Specify and pin exact versions for all required tools: Rust toolchain, C++ compilers, CMake, Qt SDK, and GPU driver development kits (CUDA SDK, ROCm).</item>
                        <item name="Local Security Tools">Mandate the setup of local security tools, such as IDE plugins for static analysis and `cargo-audit`.</item>
                    </sub_action>
                    <sub_action name="Conduct initial, mandatory security awareness and secure coding training">
                        <item name="Session Content">Cover the OWASP Top 10, common language-specific pitfalls (e.g., memory management in C++, unsafe blocks in Rust), and security risks specific to mining software (malicious third-party miners, insecure RPC endpoints).</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Log Completion of Task 0.1</summary>
                <log_entry>
                     <validation_method>Successfully cloned the repository on a clean machine, followed the `DEVELOPMENT_ENVIRONMENT.md` guide, and successfully built all initial "hello world" stubs. Verified that pre-commit hooks correctly block commits with formatting errors. Confirmed completion of the initial security training session by the entire founding team. All validation criteria are met.</validation_method>
                     <review_outcome>Approved.</review_outcome>
                </log_entry>
                <git_commit>
                    <commands>
                        <command>git add daemon/ client/ pool/ libs/ protos/ tools/ infra/ docs/DEVELOPMENT_ENVIRONMENT.md .gitignore</command>
                        <command>git commit -m "Phase 0.1: Initial Monorepo & Project Structures (Rust Daemon, C++/Qt Client), Security-Aware Dev Onboarding & Training."</command>
                        <command>git push origin develop</command>
                    </commands>
                    <remote_repository>https://github.com/emiliancristea/bunker-miner.git</remote_repository>
                    <branch>develop</branch>
                </git_commit>
            </action_item>
        </actions>

        <design_rationale>
            A clean monorepo structure is essential for managing the complexity of a multi-language project, simplifying dependency management and build orchestration. Enforced pre-commit hooks catch errors at the earliest possible stage, reducing CI load and preventing flawed code from ever being committed. A single, unified development environment document eliminates "works on my machine" issues and dramatically accelerates the onboarding of new developers.
        </design_rationale>

        <operational_considerations>
            <item name="Code Quality Gates">The pre-commit hooks and protected branch strategy will be the primary gatekeepers of code quality and consistency for the project's entire lifecycle.</item>
            <item name="Onboarding Efficiency">The `DEVELOPMENT_ENVIRONMENT.md` document will be the single source of truth for setting up a developer's machine, ensuring consistency and reducing setup friction to a minimum.</item>
        </operational_considerations>

        <validation_criteria>
            - A new developer can successfully clone the repo, follow the `DEVELOPMENT_ENVIRONMENT.md` guide, and build all initial components.
            - Pre-commit hooks are functional and prevent commits with formatting/linting errors.
            - The initial security training session is completed by the entire team, confirmed by an attendance list.
            - The Git monorepo is pushed to the remote with the correct directory structure and branch protection rules in place.
        </validation_criteria>
        
        <testing_methodologies>
            <item name="System Test">A developer successfully runs the "hello world" tests or stubs, confirming the environment is correctly configured.</item>
            <item name="Manual Validation">Manually attempting to commit incorrectly formatted code to verify that the pre-commit hooks block the action.</item>
        </testing_methodologies>
        
        <version_control_strategy>
            <item name="Branching">Gitflow (main, develop, feature/, etc.). `main` and `develop` are protected.</item>
            <item name="Commits">All initial project skeletons, directory structures, and documentation are committed to the `develop` branch. The Git Commit message for this task will be exactly as specified.</item>
        </version_control_strategy>
        
        <security_audit_checkpoints>
            <checkpoint>The Security Lead must review and approve the content of the security training materials.</checkpoint>
            <checkpoint>The enforcement of security-related pre-commit hooks (e.g., linters that catch common pitfalls) must be reviewed and approved.</checkpoint>
        </security_audit_checkpoints>
    </task>
</prompt>