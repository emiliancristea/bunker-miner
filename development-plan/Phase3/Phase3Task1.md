<prompt>
    <system_prompt>
        You are the Lead Principal Engineer, with ultimate authority on security and full-stack architecture for this project. Your immediate mission is to execute Task 3.1. The architectural plans are approved; you will now build the digital fortress where the BUNKER POOL will reside. You will personally finalize and implement the production-grade Infrastructure-as-Code (IaC) using Terraform. You will provision a secure VPC, a managed Kubernetes cluster, high-availability databases, and all necessary networking components. Your top priority is security: you will implement a defense-in-depth strategy with restrictive network policies, least-privilege IAM roles, and a default-deny security posture throughout the cloud environment. You are the sole architect and validator of this critical infrastructure. Your output will be the direct result of completing and self-validating these actions.
    </system_prompt>

    <task>
        <task_id>3.1</task_id>
        <task_title>BUNKER POOL - Core Infrastructure & IaC Setup</task_title>
        
        <technical_references>
            <reference>Terraform and AWS Provider Documentation.</reference>
            <reference>Amazon EKS, RDS, and ElastiCache Best Practices Guides.</reference>
            <reference>Finalized `docs/BUNKER_POOL_ARCHITECTURE.md`.</reference>
        </technical_references>

        <context>
            A mining pool is a complex, distributed system that requires a robust, scalable, and secure cloud infrastructure. Before we can write the pool software, we must build the environment in which it will run. This task focuses on creating the foundational, production-grade Infrastructure-as-Code (IaC) for the entire BUNKER POOL ecosystem. This translates our architectural diagrams into real, provisioned, and secured cloud resources.
        </context>

        <measurable_objectives>
            <sub_objective name="Infrastructure as Code">
                <item>Finalized, security-audited, production-grade IaC for the cloud environment (AWS) is complete and stored in the repository.</item>
            </sub_objective>
            <sub_objective name="Provisioning">
                <item>The infrastructure, including a secure VPC, a managed Kubernetes cluster (EKS), and managed databases (RDS, ElastiCache), is successfully provisioned in a dedicated staging environment.</item>
                <item>A basic "hello world" service can be successfully deployed to the new EKS cluster via the existing CI/CD pipeline, validating the entire deployment workflow.</item>
            </sub_objective>
        </measurable_objectives>

        <actions>
            <action_item>
                <summary>Finalize and Implement Production-Grade IaC</summary>
                <details>
                    <sub_action name="Finalize Terraform scripts in `/infra/terraform/production/`">
                        <item name="VPC & Networking">Define a Virtual Private Cloud (VPC) with distinct public and private subnets across multiple availability zones for high availability.</item>
                        <item name="Kubernetes Cluster">Provision a managed Amazon EKS cluster with auto-scaling node groups to handle variable load.</item>
                        <item name="Managed Databases">
                            <ul>
                                <li>Provision an Amazon RDS for PostgreSQL instance in a multi-AZ configuration for the payouts database.</li>
                                <li>Provision an Amazon ElastiCache for Redis instance for share processing and caching.</li>
                            </ul>
                        </item>
                        <item name="Networking">Set up Application Load Balancers (ALBs) and a Kubernetes Ingress controller to manage external traffic to the future API and web dashboard.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Implement and Enforce IaC Security Baseline</summary>
                <details>
                    <sub_action name="Implement defense-in-depth security directly within the Terraform code">
                        <item name="Restrictive Security Groups">Implement a default-deny policy. Database security groups must only allow traffic from the specific Kubernetes cluster security group, and nothing else.</item>
                        <item name="Least-Privilege IAM Roles">Define granular IAM roles for the EKS nodes and, using IRSA (IAM Roles for Service Accounts), for the specific pods that will need AWS API access (e.g., the payout engine).</item>
                        <item name="Kubernetes NetworkPolicies">Create default-deny NetworkPolicies at the namespace level in the Kubernetes manifests. Explicitly whitelist required communication paths (e.g., allow the Share Processor to talk to Redis, but not to the Payouts DB).</item>
                        <item name="Private Subnets">Ensure all critical components, especially the databases and most application pods, reside in private subnets with no direct internet access.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Validate Infrastructure Deployment</summary>
                <details>
                    <sub_action name="Provision the staging environment and validate the deployment pipeline">
                        <item name="Provision Staging">Execute `terraform apply` to build the full infrastructure stack in a dedicated "staging" AWS account.</item>
                        <item name="CI/CD Integration">Enhance the existing CI/CD pipeline from Phase 0 to allow deploying a simple "hello-world" application to the new staging EKS cluster.</item>
                        <item name="Validation">Successfully deploy the test application and verify that it is accessible and that the network policies are correctly preventing unauthorized communication.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Log Completion of Task 3.1</summary>
                <log_entry>
                     <validation_method>Executed a `terraform plan` which was peer-reviewed and approved. Executed `terraform apply` to successfully provision the complete infrastructure stack in our staging AWS account. Enhanced the CI/CD pipeline and performed a successful test deployment of a "hello-world" service to the new EKS cluster. Verified via `kubectl` and AWS Console that all security groups and network policies were active and correctly configured. All validation criteria are met.</validation_method>
                     <review_outcome>Approved.</review_outcome>
                </log_entry>
                <git_commit>
                    <commands>
                        <command>git add infra/terraform/production/ .github/workflows/deploy-infra.yml</command>
                        <command>git commit -m "Phase 3.1: Production IaC for BUNKER POOL Finalized and Validated."</command>
                        <command>git push origin develop</command>
                    </commands>
                    <remote_repository>https://github.com/emiliancristea/bunker-miner.git</remote_repository>
                    <branch>develop</branch>
                </git_commit>
            </action_item>
        </actions>

        <design_rationale>
            Using Infrastructure-as-Code is a non-negotiable principle for modern, reliable systems. It makes our production environment repeatable, auditable, and version-controlled. By defining security rules directly in the code (Security Groups, IAM Roles, NetworkPolicies), we make security an automated and verifiable part of the provisioning process, rather than a manual checklist item that can be forgotten. Building a dedicated staging environment that is a 1:1 mirror of production is the only way to test our software and operational procedures with high confidence.
        </design_rationale>

        <operational_considerations>
            <item name="Cost Management">This task provisions a significant amount of cloud resources. The staging environment will be the primary driver of our pre-launch cloud costs, and its resources should be monitored and scaled appropriately.</item>
            <item name="State Management">The Terraform state file is now a critical piece of infrastructure. It must be stored securely (e.g., in a versioned and encrypted S3 bucket) with state locking (e.g., via DynamoDB) to prevent concurrent modifications.</item>
        </operational_considerations>

        <validation_criteria>
            - A successful `terraform apply` provisions the full, specified infrastructure in a staging AWS account.
            - All defined security controls (Security Groups, IAM roles, NetworkPolicies) are active and correctly configured.
            - The CI/CD pipeline can successfully deploy a test application to the staging EKS cluster.
            - The entire IaC codebase has been peer-reviewed and signed off by the Security and SRE/DevOps leads.
        </validation_criteria>
        
        <testing_methodologies>
            <item name="Infrastructure Testing">Validating the IaC by actually provisioning the resources and running health checks against them.</item>
            <item name="Deployment Testing">Performing a live deployment of a simple application to validate the entire CI/CD toolchain and the Kubernetes cluster configuration.</item>
        </testing_methodologies>
        
        <version_control_strategy>
            <item name="Branching">All Terraform and Kubernetes manifest changes are developed on a `feature/pool-iac` branch.</item>
            <item name="Commits">The Git Commit message for this task will be exactly as specified.</item>
        </version_control_strategy>
        
        <security_audit_checkpoints>
            <checkpoint>A mandatory, line-by-line security review of all production-grade IaC (Terraform and Kubernetes manifests) is required before this task can be completed. The Security Lead's sign-off is non-negotiable.</checkpoint>
            <checkpoint>The implementation of least-privilege IAM roles and default-deny network policies is a critical security validation gate.</checkpoint>
        </security_audit_checkpoints>
    </task>
</prompt>