<prompt>
    <system_prompt>
        You are the Lead Principal Engineer, with ultimate authority on security and full-stack architecture for this project. Your immediate mission is to execute Task 0.4. With the core technologies and API contracts defined, you will now construct the project's automated quality and security backbone. You will personally design and implement a comprehensive Continuous Integration and Continuous Deployment (CI/CD) pipeline using GitHub Actions. This pipeline must automate the build, test, and security scanning processes for all components, including the Rust daemon and the C++/Qt client. You will enforce your security standards by integrating automated scanning tools that will fail the build on critical findings, ensuring no insecure or low-quality code can be merged. You are the sole executor and validator of this automated system. Your output will be the direct result of completing and self-validating these actions.
    </system_prompt>

    <task>
        <task_id>0.4</task_id>
        <task_title>CI/CD Pipeline Setup (Comprehensive, Functional, with Integrated Security Scanning)</task_title>
        
        <technical_references>
            <reference>GitHub Actions Documentation.</reference>
            <reference>Docker Documentation.</reference>
            <reference>`cargo`, `cmake`, `trivy`, `cargo-audit` CLI documentation.</reference>
            <reference>docs/PROJECT_GOVERNANCE_AND_WORKFLOWS.md (specifically the SDL Charter).</reference>
        </technical_references>

        <context>
            With our core technologies and APIs defined, we must establish an automated, repeatable, and secure process for building, testing, and packaging our software. Manual building is slow, prone to human error, and cannot scale. A comprehensive CI/CD pipeline is essential to enforce our quality and security standards on every single code change, providing rapid feedback to developers and ensuring the integrity of our software artifacts.
        </context>

        <measurable_objectives>
            <sub_objective name="Pipeline Functionality">
                <item>Fully functional CI workflows for the Rust daemon and the C++/Qt client are implemented in GitHub Actions.</item>
                <item>The pipelines correctly build and run unit tests for both Windows and Linux environments.</item>
            </sub_objective>
            <sub_objective name="Security Integration">
                <item>All pipelines include mandatory, automated security scanning stages (SAST, dependency, container) that will fail the build on critical findings.</item>
                <item>Protected branch rules on GitHub are configured to require all relevant CI jobs to pass before a Pull Request can be merged into `develop` or `main`.</item>
            </sub_objective>
        </measurable_objectives>

        <actions>
            <action_item>
                <summary>Implement CI Workflow for Rust Daemon</summary>
                <details>
                    <sub_action name="Create `daemon-ci.yml` in `.github/workflows/`">
                        <item name="Trigger">On push to `develop` and `feature/*` branches, and on Pull Requests targeting `develop`.</item>
                        <item name="Matrix Build">Use a build matrix for `ubuntu-latest` and `windows-latest` runners.</item>
                        <item name="Jobs">
                            <ol>
                                <li>**Setup Environment:** Install the pinned Rust toolchain and `protoc` compiler.</li>
                                <li>**Lint & Format Check:** Run `cargo fmt --check` and `cargo clippy -- -D warnings`.</li>
                                <li>**Build:** Run `cargo build --release`.</li>
                                <li>**Unit & Integration Tests:** Run `cargo test --release`.</li>
                                <li>**Security Scans (Rust):**
                                    <ul>
                                        <li>**Dependency Vulnerabilities:** Run `cargo audit --deny warnings` to check for vulnerable crate versions.</li>
                                        <li>**License Compliance:** Run `cargo deny check license` against an approved license list (e.g., MIT, Apache-2.0).</li>
                                    </ul>
                                </li>
                                <li>**Dockerize Daemon:** Build a versioned Docker image using a multi-stage Dockerfile. The final image must be minimal (e.g., `distroless` or `alpine`) and run the daemon as a non-root user.</li>
                                <li>**Container Vulnerability Scan:** Use `trivy` to scan the built Docker image for OS and application-level vulnerabilities. Fail the build if critical or high-severity vulnerabilities are found.</li>
                            </ol>
                        </item>
                    </sub_action>
                </details>
            </action_item>
            
            <action_item>
                <summary>Implement CI Workflow for C++/Qt Client</summary>
                <details>
                    <sub_action name="Create `client-ci.yml` in `.github/workflows/`">
                        <item name="Trigger">On push/PR to relevant paths in `/client/`.</item>
                        <item name="Matrix Build">Use a matrix for `ubuntu-latest` and `windows-latest` runners.</item>
                        <item name="Jobs">
                            <ol>
                                <li>**Setup Environment:** Install the pinned C++ compiler, CMake, and the specific Qt SDK version.</li>
                                <li>**Lint & Format Check:** Run `clang-format --check`.</li>
                                <li>**Build:** Run the `cmake` build process for the C++ client, which will include linking the gRPC-generated stubs.</li>
                                <li>**Unit Tests:** Run the C++ unit tests (if any at this stage).</li>
                                <li>**(Optional) Security Scans (C++):** Integrate basic C++ static analysis (e.g., `cppcheck` or Clang Static Analyzer).</li>
                            </ol>
                        </item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Configure CI Best Practices and Repository Rules</summary>
                <details>
                    <sub_action name="Optimize and Secure Pipelines">
                        <item name="Caching">Implement caching for Rust dependencies (`cargo` build artifacts), C++ build artifacts, and Docker layers to accelerate CI runs.</item>
                        <item name="Secrets Management">Use GitHub Actions encrypted secrets for any tokens needed during CI (e.g., container registry push credentials for future release builds).</item>
                    </sub_action>
                    <sub_action name="Enforce CI Checks on Pull Requests">
                        <item name="Branch Protection">Configure branch protection rules for `develop` and `main` to require all CI jobs (build, test, and security scans) to pass before merging is allowed.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Log Completion of Task 0.4</summary>
                <log_entry>
                     <validation_method>Successfully triggered the CI pipelines via a test Pull Request. Verified that all jobs, including build, test, and all security scans (`cargo audit`, `trivy`), completed successfully for both Windows and Linux. Confirmed that the PR was blocked from merging until all checks passed. Initial findings from security scans on boilerplate code were documented and triaged. All validation criteria are met.</validation_method>
                     <review_outcome>Approved.</review_outcome>
                </log_entry>
                <git_commit>
                    <commands>
                        <command>git add .github/workflows/daemon-ci.yml .github/workflows/client-ci.yml daemon/Dockerfile</command>
                        <command>git commit -m "Phase 0.4: Functional CI/CD Pipelines with Integrated Security Scanning."</command>
                        <command>git push origin develop</command>
                    </commands>
                    <remote_repository>https://github.com/emiliancristea/bunker-miner.git</remote_repository>
                    <branch>develop</branch>
                </git_commit>
            </action_item>
        </actions>

        <design_rationale>
            A comprehensive CI/CD pipeline is the heart of a modern DevOps culture. It provides rapid feedback to developers, enforces quality gates automatically, and ensures that what is tested is what gets deployed. Integrating security scanning directly into the pipeline ("shifting left") makes security a continuous, automated process, not an afterthought. This ensures that no code that is known to be vulnerable or that violates our quality standards can enter the main development branch.
        </design_rationale>

        <operational_considerations>
            <item name="Pipeline as Code">The CI/CD workflows are defined as code (`.yml` files) within the repository itself, making them version-controlled, auditable, and easy to modify.</item>
            <item name="Cost and Performance">The performance and cost of CI runners will be monitored and optimized over time. Caching is the primary tool for managing this from day one.</item>
            <item name="Future Deployment">These CI pipelines will be the sole path for code to be promoted to staging and production environments in the future. Their reliability is paramount.</item>
        </operational_considerations>

        <validation_criteria>
            - Successful "green" builds for all CI workflows on a test Pull Request to `develop`.
            - All configured security scan jobs complete successfully and are able to fail the build if a critical issue is found.
            - The ability to view scan reports and logs from the CI is confirmed.
            - Branch protection rules are active and prevent merging of failing PRs.
        </validation_criteria>
        
        <testing_methodologies>
            <item name="Pipeline Testing">The pipeline's primary validation is its ability to successfully execute the project's automated tests (Unit, Integration) and fail when those tests fail.</item>
            <item name="Security Gate Testing">Intentionally introducing a vulnerable dependency to verify that the `cargo audit` job correctly fails the build and blocks the PR.</item>
        </testing_methodologies>
        
        <version_control_strategy>
            <item name="Branching">All `.github/workflows/*.yml` files are developed on `feature/` branches and merged to `develop` before being promoted to `main` to govern the repository's processes.</item>
            <item name="Commits">The Git Commit message for this task will be exactly as specified.</item>
        </version_control_strategy>
        
        <security_audit_checkpoints>
            <checkpoint>The Security and DevOps Leads must review and approve the CI/CD workflows, especially the configuration and failure thresholds for the security scanning tools (`trivy`, `cargo audit`).</checkpoint>
            <checkpoint>The process for triaging and formally accepting or deferring findings from security scans must be formally documented and approved.</checkpoint>
            <checkpoint>The implementation of branch protection rules making these security scans a mandatory passing gate is a critical security checkpoint.</checkpoint>
        </security_audit_checkpoints>
    </task>
</prompt>