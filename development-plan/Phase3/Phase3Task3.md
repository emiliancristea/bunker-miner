<prompt>
    <system_prompt>
        You are the Lead Principal Engineer, with ultimate authority on security and full-stack architecture for this project. Your immediate mission is to execute Task 3.3. With the pool now able to accept and validate work, you will build the final, critical components: the Payout Engine and the Public API. You will personally architect and implement the secure, reliable Payout Engine that calculates miner rewards and orchestrates payments from the pool's hot wallet. You will also build the public-facing API that provides the transparency our users expect, delivering real-time pool and miner statistics. The security of the payout logic is paramount. You are the sole executor and validator of this trust-critical system. Your output will be the direct result of completing and self-validating these actions.
    </system_prompt>

    <task>
        <task_id>3.3</task_id>
        <task_title>BUNKER POOL - Payout Engine & API</task_title>
        
        <technical_references>
            <reference>PPLNS (Pay Per Last N Shares) reward calculation scheme documentation.</reference>
            <reference>`axum` crate documentation for REST APIs.</reference>
            <reference>PostgreSQL and `sqlx` crate documentation.</reference>
            <reference>RPC documentation for coin daemon wallet functions (e.g., `sendmany`).</reference>
            <reference>Finalized `docs/BUNKER_POOL_ARCHITECTURE.md`.</reference>
        </technical_references>

        <context>
            A mining pool is only useful if it reliably and fairly pays its miners. This task focuses on building the engine that calculates miner rewards based on their submitted work and the public-facing API that exposes pool and miner statistics. The Payout Engine is the most financially sensitive part of the entire ecosystem, and its security, accuracy, and reliability are non-negotiable.
        </context>

        <measurable_objectives>
            <sub_objective name="Payout Engine">
                <item>A functional Payout Engine is implemented that correctly calculates miner rewards based on a PPLNS scheme.</item>
                <item>The engine can securely construct and submit a batch payment transaction to the coin daemon to pay out multiple miners at once.</item>
                <item>The entire process is transactional and resilient to failures, ensuring no funds are lost or double-spent.</item>
            </sub_objective>
            <sub_objective name="API Server">
                <item>A public-facing REST and WebSocket API is implemented and deployed.</item>
                <item>The API successfully provides real-time public pool statistics and per-miner performance data.</item>
            </sub_objective>
        </measurable_objectives>

        <actions>
            <action_item>
                <summary>Implement the Payout Engine</summary>
                <details>
                    <sub_action name="Develop the payout logic as a background worker in `/pool/payout_engine/`">
                        <item name="Trigger">The engine will be a Kubernetes `CronJob` or a long-running worker that is triggered when the pool's Share Processor detects that a new block has been found by the pool.</item>
                        <item name="Reward Calculation (PPLNS)">
                            <ol>
                                <li>When a block is found, the engine retrieves the last N shares from the Redis `round:{...}:shares` sorted set.</li>
                                <li>It calculates the total difficulty of these N shares.</li>
                                <li>For each miner who contributed to those shares, it calculates their percentage of the total difficulty.</li>
                                <li>The block reward (minus the pool fee) is distributed to the miners according to this percentage.</li>
                            </ol>
                        </item>
                        <item name="Transactional Database Logic">
                            <ul>
                                <li>The calculated reward amounts (`amount_owed`) are written to a `payouts` table in the PostgreSQL database with a status of `UNPAID`. This must be done within a single database transaction.</li>
                            </ul>
                        </item>
                        <item name="Secure Payment Execution">
                            <ul>
                                <li>A separate, highly secured process will periodically scan the `payouts` table for `UNPAID` balances that have met the minimum payout threshold.</li>
                                <li>It will group these payouts and use the coin daemon's `sendmany` RPC command to construct and send a single, atomic batch transaction to the network.</li>
                                <li>Upon successful broadcast of the transaction, it will update the status of the corresponding rows in the `payouts` table to `PAID`, storing the transaction ID.</li>
                            </ul>
                        </item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Implement the Public API Server</summary>
                <details>
                    <sub_action name="Develop the API server in `/pool/api_server/` using `axum`">
                        <item name="REST Endpoints">
                            <ul>
                                <li>`GET /api/pool/stats`: Returns public pool data (total hashrate, active miners, blocks found), queried from Redis and PostgreSQL.</li>
                                <li>`GET /api/miners/{walletAddress}/stats`: Returns a specific miner's data (current hashrate, unpaid balance, payment history), queried from PostgreSQL.</li>
                            </ul>
                        </item>
                        <item name="WebSocket Endpoint">
                            <ul>
                                <li>Implement a `/ws` endpoint that streams real-time data, such as newly found blocks and public pool hashrate updates.</li>
                            </ul>
                        </item>
                        <item name="Deployment">Containerize the API server and deploy it to the staging Kubernetes cluster behind the Ingress controller.</item>
                    </sub_action>
                </details>
            </action_item>
            
            <action_item>
                <summary>Implement Security Hardening for Payouts</summary>
                <details>
                    <sub_action name="Isolate and secure the pool's hot wallet">
                        <item name="Secret Management">The private keys or wallet password for the pool's hot wallet must be stored in a dedicated, highly secure secret management system (e.g., AWS Secrets Manager or HashiCorp Vault), NOT in Kubernetes secrets.</item>
                        <item name="Process Isolation">The payment execution process must run in its own isolated pod with the most restrictive permissions possible. It should only have network access to the database and the coin daemon RPC port.</item>
                        <item name="Manual Approval Gate">Implement a mandatory manual approval step for any withdrawal transaction that exceeds a certain threshold (e.g., > 1 BTC equivalent). The payment process will flag the transaction for review, and an admin must manually approve it via a secure internal tool before it can be broadcast.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Log Completion of Task 3.3</summary>
                <log_entry>
                     <validation_method>Conducted an integration test in the staging environment. After simulating a block find by the pool, verified that the Payout Engine correctly calculated and recorded PPLNS rewards for all participating test miners in the PostgreSQL database. The payment execution process successfully created and broadcast a batch transaction. The Public API correctly reported the unpaid and then paid balances for the test miners. All validation criteria are met.</validation_method>
                     <review_outcome>Approved.</review_outcome>
                </log_entry>
                <git_commit>
                    <commands>
                        <command>git add pool/src/payout_engine.rs pool/src/public_api.rs pool/Cargo.toml</command>
                        <command>git commit -m "Phase 3.3: Implemented BUNKER POOL Payout Engine & Public API."</command>
                        <command>git push origin develop</command>
                    </commands>
                    <remote_repository>https://github.com/emiliancristea/bunker-miner.git</remote_repository>
                    <branch>develop</branch>
                </git_commit>
            </action_item>
        </actions>

        <design_rationale>
            The separation of reward calculation from payment execution is a critical security and reliability pattern. It allows the high-frequency reward calculation to happen quickly and be recorded in a transactional database. The slower, more sensitive process of interacting with the hot wallet is done by a separate, more isolated, and lower-frequency process. The PPLNS scheme is a standard, fair method that rewards loyal miners. The public API provides the transparency that is essential for building trust with the mining community.
        </design_rationale>

        <operational_considerations>
            <item name="Hot Wallet Management">The security and funding of the pool's hot wallet is the single most critical operational responsibility. The wallet must be continuously monitored for suspicious activity, and the manual approval process for large withdrawals is a non-negotiable safety measure.</item>
            <item name="Database Integrity">The PostgreSQL database is the source of truth for all financial records. It must have a robust backup and restore plan, as validated in our DR drills.</item>
        </operational_considerations>

        <validation_criteria>
            - After a simulated block find, the Payout Engine correctly calculates and records rewards for all contributing miners.
            - The payment execution process can successfully create and broadcast a batch payment transaction.
            - The Public API correctly reports pool-wide and per-miner statistics, including unpaid balances.
            - The security architecture for protecting the hot wallet is implemented and peer-reviewed.
        </validation_criteria>
        
        <testing_methodologies>
            <item name="Integration Testing">The primary validation is a full integration test of the entire payout lifecycle: from a simulated block find, through the PPLNS calculation, to the database record, and finally to a successful broadcast of a transaction from the hot wallet on a testnet.</item>
            <item name="Unit Testing">Exhaustive unit tests will be written for the PPLNS reward calculation logic to verify its mathematical correctness with various edge cases.</item>
        </testing_methodologies>
        
        <version_control_strategy>
            <item name="Branching">The payout engine and API are developed on a `feature/pool-payouts-api` branch.</item>
            <item name="Commits">The Git Commit message for this task will be exactly as specified.</item>
        </version_control_strategy>
        
        <security_audit_checkpoints>
            <checkpoint>A mandatory, exhaustive security review of the entire Payout Engine is required. This is the most critical security review of Phase 3. The review must focus on the security of the hot wallet, the prevention of unauthorized transactions, and the correctness of the financial calculations to prevent loss of funds.</checkpoint>
            <checkpoint>The implementation of the manual approval gate for large withdrawals must be validated.</checkpoint>
            <checkpoint>The Public API must be reviewed to ensure it does not leak any private user data and is not vulnerable to denial-of-service attacks.</checkpoint>
        </security_audit_checkpoints>
    </task>
</prompt>