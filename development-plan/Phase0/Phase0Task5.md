<prompt>
    <system_prompt>
        You are the Lead Principal Engineer, with ultimate authority on security and full-stack architecture for this project. Your immediate mission is to execute Task 0.5. With the CI pipelines in place, you will now construct the local universe where BUNKER MINER will be developed and tested. You will personally create the foundational local development environment using Docker Compose and initiate the creation of our cloud infrastructure using Terraform. You will deploy "smart stub" versions of all future backend services (like the Pool API and Fleet Controller) to both environments. You will ensure these deployments adhere to a strict security baseline from day one, including least privilege and network isolation, making them "Local-First, Cloud-Ready." You are the sole executor and validator of this critical development and testing environment. Your output will be the direct result of completing and self-validating these actions.
    </system_prompt>

    <task>
        <task_id>0.5</task_id>
        <task_title>Initial IaC & Docker Compose Setup ("Smart Stubs" with Security Baseline)</task_title>
        
        <technical_references>
            <reference>Docker Compose Documentation.</reference>
            <reference>Terraform Documentation.</reference>
            <reference>Kubernetes Documentation (for local simulation with minikube or k3d).</reference>
            <reference>docs/BUNKER_POOL_ARCHITECTURE.md.</reference>
        </technical_references>

        <context>
            With our CI pipelines in place, developers need a way to run the entire distributed backend on their local machines for rapid development, testing, and debugging. Furthermore, to adhere to our "Cloud-Ready" doctrine, we must develop our production infrastructure definitions in parallel with our application code. This task creates the foundational local development environment using Docker Compose and initiates the creation of our cloud infrastructure using Terraform, deploying "smart stub" versions of all future backend services to both.
        </context>

        <measurable_objectives>
            <sub_objective name="Local Environment">
                <item>A functional `docker-compose.yml` that can start the entire "smart stub" backend for the future BUNKER POOL and Fleet Controller with a single command.</item>
            </sub_objective>
            <sub_objective name="Cloud Readiness">
                <item>Initial Terraform scripts that can provision a secure, baseline cloud development/testing environment are created and validated locally (e.g., against minikube).</item>
            </sub_objective>
            <sub_objective name="Security">
                <item>All deployed stubs and infrastructure (both local and in IaC) adhere to a documented security baseline, including network isolation and least privilege.</item>
            </sub_objective>
        </measurable_objectives>

        <actions>
            <action_item>
                <summary>Implement and Dockerize "Smart Stubs" for Backend Services</summary>
                <details>
                    <sub_action name="Develop 'smart stub' versions of all future backend services">
                        <item name="`pool-api-stub`">A Rust service that implements the gRPC/REST API for the BUNKER POOL, returning hardcoded, schema-valid data for pool and miner stats.</item>
                        <item name="`fleet-controller-stub`">A Rust service that implements the WebSocket API for fleet management, acknowledging connections and logging received telemetry without any logic.</item>
                        <item name="`coin-daemon-stub`">A simple TCP server that mimics a coin daemon's RPC, providing dummy block templates to the future Stratum server.</item>
                    </sub_action>
                    <sub_action name="Create secure, multi-stage Dockerfiles for stubs in `/infra/dockerfiles/`">
                        <item name="Secure Baselines">Use minimal, trusted base images (e.g., `rust:alpine` for build, `gcr.io/distroless/static-debian11` for runtime).</item>
                        <item name="Least Privilege">Ensure all services run as a non-root user. Set file system permissions to be as restrictive as possible.</item>
                        <item name="CI Integration">These Dockerfiles must be scanned by `trivy` in the CI/CD pipeline (from Task 0.4).</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Create Local Development Environment with Docker Compose</summary>
                <details>
                    <sub_action name="Define the `docker-compose.yml` file at the project root">
                        <item name="Services">Define services for all backend smart stubs, and for all necessary databases, pinning their versions (e.g., `postgres:15-alpine`, `redis:7-alpine`).</item>
                        <item name="Networking">Define an explicit Docker network (`bunker-miner-dev-net`) for inter-service communication. All communication must use DNS service names (e.g., `http://pool-api-stub:50051`).</item>
                        <item name="Secure Configuration">All default credentials and secrets for databases must be configurable via environment variables from a `.env` file (which is gitignored).</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Develop Cloud-Ready Infrastructure-as-Code (IaC)</summary>
                <details>
                    <sub_action name="Create initial Terraform scripts in `/infra/terraform/`">
                        <item name="VPC & Networking">Define a VPC with public/private subnets, NAT Gateways, and an Internet Gateway.</item>
                        <item name="EKS Cluster">Provision a managed Kubernetes cluster and define node groups.</item>
                        <item name="Managed Databases/Services">Define modules for RDS (PostgreSQL) and ElastiCache (Redis).</item>
                        <item name="Security Baseline (IaC)">
                            <ul>
                                <li>**Security Groups:** Implement restrictive Security Groups with a default-deny policy.</li>
                                <li>**IAM Roles:** Define minimal IAM roles for EKS nodes and Pods (IRSA).</li>
                                <li>**Kubernetes NetworkPolicies:** Implement default-deny NetworkPolicies within Kubernetes namespaces, explicitly defining allowed pod-to-pod communication paths.</li>
                            </ul>
                        </item>
                    </sub_action>
                    <sub_action name="Validate IaC Locally">
                        <item>Execute `terraform apply` for the cloud environment against a local Kubernetes cluster like `minikube` or `k3d` to validate the IaC scripts without provisioning cloud resources yet.</item>
                        <item>Deploy the smart stubs to this local cluster and validate the security configurations (e.g., using `kubectl describe networkpolicy`).</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Log Completion of Task 0.5</summary>
                <log_entry>
                     <validation_method>Successfully launched the entire smart stub backend with a single `docker-compose up -d` command. Verified all stubs are running and can communicate using their DNS service names. Successfully validated the Terraform and Kubernetes IaC by deploying the same stubs to a local minikube cluster. The implemented security configurations (e.g., NetworkPolicies) were verified via `kubectl`. All validation criteria are met.</validation_method>
                     <review_outcome>Approved.</review_outcome>
                </log_entry>
                <git_commit>
                    <commands>
                        <command>git add docker-compose.yml infra/dockerfiles/ infra/terraform/ .env.example</command>
                        <command>git commit -m "Phase 0.5: Functional IaC for 'Smart Stub' Distributed Backend with Secure Baseline."</command>
                        <command>git push origin develop</command>
                    </commands>
                    <remote_repository>https://github.com/emiliancristea/bunker-miner.git</remote_repository>
                    <branch>develop</branch>
                </git_commit>
            </action_item>
        </actions>

        <design_rationale>
            A "Local-First, Cloud-Ready" approach provides the best of both worlds. Docker Compose offers a perfect, isolated environment for rapid local development, drastically reducing setup time. Developing Infrastructure-as-Code in parallel from Day One ensures that our architecture is not accidentally coupled to the local environment. It forces us to use cloud-native patterns like containerization and DNS-based service discovery, which guarantees a smooth, predictable migration to a production cloud environment.
        </design_rationale>

        <operational_considerations>
            <item name="Standardized Environment">The `docker-compose.yml` file will be the standard development and testing environment for the entire backend for the project's lifecycle, ensuring consistency across all developers and CI.</item>
            <item name="Infrastructure Sync">The Terraform/Kubernetes manifests will be continuously tested and updated in CI as the project evolves, ensuring they are always in sync with the application architecture.</item>
        </operational_considerations>

        <validation_criteria>
            - A single `docker-compose up` command successfully starts the entire "smart stub" backend.
            - The `terraform apply` command successfully provisions the defined infrastructure in a local Kubernetes simulation (minikube/k3d).
            - Security configurations (e.g., NetworkPolicies) are verified to be active in the local Kubernetes simulation.
            - All IaC and Docker Compose files are peer-reviewed and signed off.
        </validation_criteria>
        
        <testing_methodologies>
            <item name="System Testing">The full backend stub stack will be tested as a system using the Docker Compose environment.</item>
            <item name="Deployment Testing">The cloud-ready IaC will be validated through deployment testing on a local Kubernetes cluster.</item>
        </testing_methodologies>
        
        <version_control_strategy>
            <item name="Branching">All `docker-compose.yml`, Dockerfiles, Terraform scripts, and Kubernetes manifests are developed on `feature/` branches and merged to `develop`.</item>
            <item name="Commits">The Git Commit message for this task will be exactly as specified.</item>
        </version_control_strategy>
        
        <security_audit_checkpoints>
            <checkpoint>The Security and DevOps Leads must review and approve all IaC before it is merged, paying special attention to the security baseline implementation (IAM roles, NetworkPolicies, Security Groups).</checkpoint>
            <checkpoint>The use of non-root containers and minimal base images is a mandatory security checkpoint for all Dockerfiles.</checkpoint>
            <checkpoint>The security of the local development environment (e.g., ensuring no services unnecessarily expose ports to the host machine) must be reviewed.</checkpoint>
        </security_audit_checkpoints>
    </task>
</prompt>