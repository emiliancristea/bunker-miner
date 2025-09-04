<prompt>
    <system_prompt>
        You are the Lead Principal Engineer, with ultimate authority on security and full-stack architecture for this project. Your immediate mission is to execute Task 5.1. With the architecture approved, you will now build the economic engine of our peer-to-peer ecosystem. You will personally architect and implement the backend for the Hashpower Marketplace. This includes extending the database schema, building the API for buyers to place orders, engineering the low-latency matching engine that assigns sellers to orders, and creating the secure, transactional payout system. The financial integrity of this system is your absolute priority. You are the sole executor and validator of this complex, mission-critical service. Your output will be the direct result of completing and self-validating these actions.
    </system_prompt>

    <task>
        <task_id>5.1</task_id>
        <task_title>Hashpower Marketplace - Backend Matching Engine & Payouts</task_title>
        
        <technical_references>
            <reference>Finalized ADR for Hashpower Marketplace Logic.</reference>
            <reference>PostgreSQL and `sqlx` crate documentation for transactional queries.</reference>
            <reference>BUNKER POOL share processing logic (for reference on share validation).</reference>
        </technical_references>

        <context>
            The core of a hashpower marketplace is the backend system that allows buyers to place orders for hashrate and sellers (miners) to fulfill them. This requires a highly available, low-latency matching engine and a secure, transactional payout system to ensure buyers get the hashrate they paid for and sellers get paid for their work. This task focuses on building this complex, financially sensitive backend service.
        </context>

        <measurable_objectives>
            <sub_objective name="Backend Service">
                <item>The PostgreSQL database is extended with secure schemas for `hash_orders` and `hash_deliveries`.</item>
                <item>A new `Marketplace` service is implemented that can accept, validate, and manage hashpower orders from buyers.</item>
                <item>A matching engine is implemented that can intelligently assign opted-in daemons (sellers) to fulfill active orders.</item>
            </sub_objective>
            <sub_objective name="Financial Integrity">
                <item>A secure payout system is functional, which correctly calculates and distributes payments to sellers based on valid, delivered shares, while debiting the buyer's account.</item>
                <item>The entire process is transactional to prevent loss of funds for either party.</item>
            </sub_objective>
        </measurable_objectives>

        <actions>
            <action_item>
                <summary>Extend Database and Implement Order Management API</summary>
                <details>
                    <sub_action name="Enhance the PostgreSQL database schema using `sqlx-cli` migrations">
                        <item name="`hash_orders` table">Columns for `order_id`, `buyer_user_id`, `algorithm`, `pool_details` (JSONB for host, port, user, pass), `price_per_terahash_eur` (numeric), `speed_limit_mhs` (bigint), `total_budget_eur` (numeric), `status` (enum: active, fulfilled, cancelled).</item>
                        <item name="`hash_deliveries` table">Columns for `delivery_id`, `order_id`, `seller_rig_id`, `start_time`, `end_time`, `accepted_shares_count` (bigint), `total_difficulty` (numeric).</item>
                    </sub_action>
                    <sub_action name="Implement the `Marketplace` service backend">
                        <item name="Order API">Expose secure, authenticated gRPC/REST endpoints for buyers to `CreateOrder`, `ViewOrderStatus`, and `CancelOrder`. The `CreateOrder` endpoint must perform a transactional check to verify the buyer has sufficient funds in their account balance and place a hold on the `total_budget`.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Implement the Matching Engine and Payout System</summary>
                <details>
                    <sub_action name="Build the Matching Engine logic">
                        <item name="Core Loop">This will be a background worker. It will periodically scan for active `hash_orders` that have less than their `speed_limit_mhs` assigned.</item>
                        <item name="Seller Selection">It will query the `Fleet Controller` for a list of available rigs that are opted-in to selling hashpower and support the required algorithm.</item>
                        <item name="Job Assignment">The engine will send a `START_MINING_FOR_ORDER` command via the Fleet Controller's WebSocket to the selected rig, providing the buyer's pool details and a unique `delivery_id`.</item>
                    </sub_action>
                    <sub_action name="Integrate Share Logging">
                        <item name="Share Tagging">The Stratum server will be enhanced. When a miner is working on a marketplace order, every share they submit will be tagged with the `delivery_id`.</li>
                        <item name="Share Persistence">The Share Processor will validate these tagged shares as normal, but instead of writing to the Redis round set, it will append them to the `hash_deliveries` table in PostgreSQL, incrementing the share count and total difficulty.</item>
                    </sub_action>
                    <sub_action name="Implement the Marketplace Payout Engine">
                        <item name="Payout Worker">This is a new, transactional background worker that runs periodically (e.g., every 10 minutes).</item>
                        <item name="Calculation">For each active `hash_delivery`, it calculates the value of the new shares submitted since the last run (`total_difficulty` * `price_per_terahash`).</item>
                        <item name="Atomic Transaction">Within a single database transaction, it will:
                            <ol>
                                <li>Debit the calculated amount from the buyer's account balance.</li>
                                <li>Credit the seller's account balance (minus the BUNKER MINER fee).</li>
                                <li>Credit the platform's fee account.</li>
                                <li>Mark the processed shares as paid.</li>
                            </ol>
                        </item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Log Completion of Task 5.1</summary>
                <log_entry>
                     <validation_method>Executed a full integration test. A test script created a buyer's order via the new API. Verified the matching engine correctly dispatched a test rig (seller) to mine to the buyer's specified pool. Shares submitted by the seller were correctly tagged and recorded in the PostgreSQL `hash_deliveries` table. The payout engine ran and successfully performed the three-way atomic transaction, correctly debiting the buyer and crediting the seller and platform fee accounts. All validation criteria are met.</validation_method>
                     <review_outcome>Approved.</review_outcome>
                </log_entry>
                <git_commit>
                    <commands>
                        <command>git add pool/src/marketplace.rs pool/src/matching_engine.rs web/marketplace/</command>
                        <command>git commit -m "Phase 5.1: Implemented Hashpower Marketplace Backend & Matching Engine."</command>
                        <command>git push origin develop</command>
                    </commands>
                    <remote_repository>https://github.com/emiliancristea/bunker-miner.git</remote_repository>
                    <branch>develop</branch>
                </git_commit>
            </action_item>
        </actions>

        <design_rationale>
            The security and transactional integrity of the marketplace are paramount. Using a robust relational database like PostgreSQL for all financial records and order states is a non-negotiable requirement. All financial operations (placing budget holds, processing payouts) must be performed within strict database transactions to prevent race conditions and ensure that funds are never lost or created, even in the event of a system crash. The separation of the matching engine, share processor, and payout engine into distinct services creates a resilient and scalable architecture.
        </design_rationale>

        <operational_considerations>
            <item name="Latency">The performance of the matching engine is key. The time between a buyer placing an order and a seller starting to mine must be minimized. This will require efficient database queries and a low-latency messaging system to the daemons.</item>
            <item name="Dispute Resolution">The system must have a clear, documented process for handling disputes, such as a buyer claiming the delivered hashrate was invalid. The immutable record of validated shares in our database will be the primary source of truth for resolving these cases.</item>
        </operational_considerations>

        <validation_criteria>
            - A buyer can successfully create a hashpower order, and the system correctly places a hold on their funds.
            - The matching engine successfully assigns an available seller rig to the order.
            - Shares submitted by the seller are correctly validated and recorded against the order.
            - The payout engine correctly and atomically processes payments for delivered shares.
        </validation_criteria>
        
        <testing_methodologies>
            <item name="Transactional Testing">The primary validation method is writing integration tests that specifically target the transactional integrity of the payout engine. Tests will attempt to force failures mid-transaction to verify that the database correctly rolls back, leaving the system in a consistent state.</item>
            <item name="Load Testing">The matching engine will be load-tested to ensure it can handle a high volume of new orders and available sellers without significant delays.</item>
        </testing_methodologies>
        
        <version_control_strategy>
            <item name="Branching">The marketplace backend services are developed on a `feature/marketplace-backend` branch.</item>
            <item name="Commits">The Git Commit message for this task will be exactly as specified.</item>
        </version_control_strategy>
        
        <security_audit_checkpoints>
            <checkpoint>A mandatory, exhaustive security review of the entire marketplace financial lifecycle is required. This must cover the order placement (budget holds), the share validation, and, most critically, the payout engine's transactional logic to prevent any form of economic exploit or loss of funds.</checkpoint>
            <checkpoint>The logic for validating a buyer's provided pool credentials must be reviewed to ensure it cannot be abused (e.g., to launch DDoS attacks).</checkpoint>
        </security_audit_checkpoints>
    </task>
</prompt>