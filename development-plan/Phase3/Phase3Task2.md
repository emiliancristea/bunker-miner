<prompt>
    <system_prompt>
        You are the Lead Principal Engineer, with ultimate authority on security and full-stack architecture for this project. Your immediate mission is to execute Task 3.2. With the cloud infrastructure in place, you will now build the heart of the BUNKER POOL. You will personally architect and implement the high-performance, multi-algorithm Stratum server in Rust, capable of communicating with thousands of concurrent miners. You will also build the secure, high-throughput share processing service that validates the work submitted by these miners. You are the sole executor and validator of this mission-critical, low-latency system, ensuring it is scalable, resilient, and secure from day one. Your output will be the direct result of completing and self-validating these actions.
    </system_prompt>

    <task>
        <task_id>3.2</task_id>
        <task_title>BUNKER POOL - Stratum Server & Share Processor</task_title>
        
        <technical_references>
            <reference>Stratum Mining Protocol V1 Specification.</reference>
            <reference>`tokio` crate documentation for high-concurrency TCP.</reference>
            <reference>`redis-rs` crate documentation for async Redis commands.</reference>
            <reference>RPC documentation for target coin daemons (e.g., Kaspa, Ravencoin).</reference>
            <reference>Finalized `docs/BUNKER_POOL_ARCHITECTURE.md`.</reference>
        </technical_references>

        <context>
            This is the core of the mining pool. The Stratum server is the high-performance, high-concurrency component that communicates directly with thousands of miners, while the share processor is the critical backend logic that validates their work. Building these components to be secure, scalable, and correct is fundamental to the pool's success and the trust of its users. This task focuses on implementing these foundational, high-throughput services.
        </context>

        <measurable_objectives>
            <sub_objective name="Stratum Server">
                <item>A functional, multi-algorithm Stratum v1 server is implemented in Rust using Tokio.</item>
                <item>The server can accept TCP connections from miners, handle the Stratum handshake (`mining.subscribe`, `mining.authorize`), and send them valid work (jobs).</item>
            </sub_objective>
            <sub_objective name="Share Processor">
                <item>A share processing service is implemented that can receive submitted shares from the Stratum server.</item>
                <item>The service correctly validates the shares against the coin daemon's requirements (e.g., difficulty) and stores valid shares in Redis.</item>
            </sub_objective>
            <sub_objective name="Performance">
                <item>The combined system can handle a simulated load of at least 1,000 concurrent miner connections in the staging environment.</item>
            </sub_objective>
        </measurable_objectives>

        <actions>
            <action_item>
                <summary>Implement High-Concurrency Stratum Server</summary>
                <details>
                    <sub_action name="Develop the Stratum server in `/pool/stratum_server/`">
                        <item name="TCP Server">Use `tokio::net::TcpListener` to build a server that can handle thousands of persistent TCP connections concurrently.</item>
                        <item name="Stratum Protocol Logic">
                            <ul>
                                <li>Implement the JSON-RPC 2.0 message parsing for the Stratum protocol.</li>
                                <li>Handle `mining.subscribe` and `mining.authorize` requests to manage miner sessions.</li>
                                <li>Handle `mining.submit` requests, which contain the completed work (shares) from miners.</li>
                            </ul>
                        </item>
                        <item name="Job Manager">
                            <ul>
                                <li>Implement a `JobManager` that periodically polls the target coin daemon's RPC (e.g., `getblocktemplate`) to get new work.</li>
                                <li>This manager will then broadcast new `mining.notify` (job) messages to all connected and subscribed miners.</li>
                            </ul>
                        </item>
                        <item name="Share Forwarding">Upon receiving a valid `mining.submit` message, the Stratum server will immediately forward the share payload to the Share Processor service via a secure, internal message bus (e.g., NATS) or a direct gRPC call.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Implement Secure Share Processor</summary>
                <details>
                    <sub_action name="Develop the share processing service in `/pool/share_processor/`">
                        <item name="Message Consumption">The service will listen for share payloads from the Stratum server.</item>
                        <item name="Share Validation Logic">This is a critical security step. For each share, the service must:
                            <ol>
                                <li>Verify the job ID is still valid (not from a stale block).</li>
                                <li>Re-calculate the hash of the share's block header.</li>
                                <li>Verify that the hash meets the required share difficulty.</li>
                                <li>Check for duplicate submissions.</li>
                            </ol>
                        </item>
                        <item name="Persistence">
                            <ul>
                                <li>Valid shares are written to a Redis sorted set for the current block round: `round:{blockHeight}:shares`. The score will be the share's difficulty, and the member will be the miner's wallet address. This data structure is optimized for the PPLNS payout calculation.</li>
                                <li>Invalid shares are logged and tracked to identify potentially misconfigured or malicious miners.</li>
                            </ul>
                        </item>
                    </sub_action>
                </details>
            </action_item>
            
            <action_item>
                <summary>Containerize and Deploy to Staging</summary>
                <details>
                    <sub_action name="Create production-grade Dockerfiles and Kubernetes manifests">
                        <item name="Containerization">Build optimized, minimal Docker images for the Stratum server and Share Processor.</item>
                        <item name="Deployment">Create Kubernetes `Deployment` and `Service` manifests. The Stratum server's service will be of type `LoadBalancer` to expose its TCP ports to the internet, while the Share Processor will be an internal-only service.</item>
                        <item name="Configuration">All sensitive information, such as coin daemon RPC credentials, will be passed to the pods via secure Kubernetes Secrets.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Log Completion of Task 3.2</summary>
                <log_entry>
                     <validation_method>Successfully deployed the Stratum Server and Share Processor to the staging EKS cluster. Configured the BUNKER MINER daemon (from Phase 2) to connect to the new pool's Stratum port. Verified the daemon successfully subscribed, received jobs, and submitted shares. Inspected the Redis database and confirmed that valid shares were being correctly recorded with the proper miner address and difficulty score. A load test simulating 1,000 concurrent miners was executed successfully. All validation criteria are met.</validation_method>
                     <review_outcome>Approved.</review_outcome>
                </log_entry>
                <git_commit>
                    <commands>
                        <command>git add pool/src/stratum_server.rs pool/src/share_processor.rs pool/Cargo.toml</command>
                        <command>git commit -m "Phase 3.2: Implemented BUNKER POOL Stratum Server & Share Processor."</command>
                        <command>git push origin develop</command>
                    </commands>
                    <remote_repository>https://github.com/emiliancristea/bunker-miner.git</remote_repository>
                    <branch>develop</branch>
                </git_commit>
            </action_item>
        </actions>

        <design_rationale>
            Separating the Stratum server (the high-concurrency "frontend") from the Share Processor (the CPU-intensive "backend") is a crucial architectural pattern for scalability and resilience. This allows us to scale each component independently. The Stratum server can be scaled out to handle more connections, while the Share Processor can be scaled up with more CPU resources to handle a higher volume of submitted work. Using Redis for share storage provides the extremely low-latency performance required for a high-throughput mining pool.
        </design_rationale>

        <operational_considerations>
            <item name="Coin Daemon Dependency">The pool is critically dependent on the stability and uptime of the full-node coin daemons it connects to. A robust, redundant setup for these daemons is essential for the pool's operation.</item>
            <item name="Network Security">The Stratum server's TCP ports will be exposed to the public internet and will be a primary target for DDoS attacks. Production deployments will require robust DDoS mitigation at the network edge (e.g., AWS Shield).</item>
        </operational_considerations>

        <validation_criteria>
            - The BUNKER MINER daemon can successfully connect to the Stratum server, receive work, and submit valid shares.
            - The Share Processor correctly validates shares and stores them in the Redis database with the correct structure.
            - The system remains stable and performant under a simulated load of 1,000 concurrent miners.
            - All components are successfully containerized and deployed to the staging Kubernetes environment.
        </validation_criteria>
        
        <testing_methodologies>
            <item name="Integration Testing">The primary validation is an end-to-end test where real BUNKER MINER daemon instances connect to the deployed pool services and perform actual mining against a testnet coin daemon.</item>
            <item name="Load Testing">Using a specialized tool (e.g., a custom Rust Stratum client harness) to simulate thousands of concurrent connections to test the scalability and stability of the Stratum server.</item>
            <item name="Unit Testing">For the share validation logic, to test all edge cases (stale shares, low-difficulty shares, duplicates).</item>
        </testing_methodologies>
        
        <version_control_strategy>
            <item name="Branching">The pool's core services are developed on a `feature/pool-core-services` branch.</item>
            <item name="Commits">The Git Commit message for this task will be exactly as specified.</item>
        </version_control_strategy>
        
        <security_audit_checkpoints>
            <checkpoint>The Security Lead must review the Stratum server implementation for potential DoS vulnerabilities (e.g., resource exhaustion from malicious connection patterns).</checkpoint>
            <checkpoint>The Share Processor's validation logic is a critical security gate to prevent miners from getting credit for invalid or fake work. This logic must be exhaustively reviewed.</checkpoint>
            <checkpoint>The security of the connection between the pool services and the coin daemon RPCs (e.g., running over a private network, using password authentication) must be validated.</checkpoint>
        </security_audit_checkpoints>
    </task>
</prompt>