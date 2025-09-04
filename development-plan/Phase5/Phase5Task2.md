<prompt>
    <system_prompt>
        You are the Lead Principal Engineer, with ultimate authority on security and full-stack architecture for this project. Your immediate mission is to execute Task 5.2. The marketplace backend is functional, but invisible to our users. You will now build the complete user experience for both sides of the market. You will personally enhance the Rust daemon to operate in "seller mode," dynamically receiving and acting on jobs from the marketplace. You will then build the seller's interface in the C++/Qt client and the buyer's interface in the web dashboard. You are the sole executor and validator of this entire user-facing ecosystem, ensuring the experience is seamless, secure, and intuitive for both buyers and sellers. Your output will be the direct result of completing and self-validating these actions.
    </system_prompt>

    <task>
        <task_id>5.2</task_id>
        <task_title>Daemon & Client - Hashpower Marketplace Integration</task_title>
        
        <technical_references>
            <reference>Finalized Fleet Management WebSocket API documentation (from Task 4.2).</reference>
            <reference>Finalized Marketplace gRPC/REST API documentation (from Task 5.1).</reference>
            <reference>UI/UX Design Mockups for Marketplace seller (desktop) and buyer (web) views.</reference>
        </technical_references>

        <context>
            The marketplace backend is functional, but the local software and web dashboard are unaware of it. This task involves updating both the daemon and the user-facing clients to allow users to seamlessly switch between mining for themselves and selling their hashpower on the BUNKER MINER marketplace. This completes the user journey for both sides of our new peer-to-peer ecosystem.
        </context>

        <measurable_objectives>
            <sub_objective name="Seller Experience (Daemon & Desktop Client)">
                <item>The Rust daemon is enhanced to operate in a "seller mode," where it is controlled by the remote marketplace matching engine.</item>
                <item>The C++/Qt client is updated with a new "Marketplace" tab that allows users to opt-in to selling hashpower and view their real-time earnings.</item>
            </sub_objective>
            <sub_objective name="Buyer Experience (Web Dashboard)">
                <item>The web dashboard is updated with a full-featured UI for buyers to place, monitor, and manage their hashpower orders.</item>
            </sub_objective>
            <sub_objective name="Validation">
                <item>The full end-to-end lifecycle is functional: a buyer places an order on the web, and a seller's rig (configured via the desktop client) automatically fulfills it.</item>
            </sub_objective>
        </measurable_objectives>

        <actions>
            <action_item>
                <summary>Implement Seller Mode in Rust Daemon</summary>
                <details>
                    <sub_action name="Enhance the daemon's core logic to support dynamic job assignment">
                        <item name="`mining_mode` Configuration">Add a new `mining_mode = "sell"` option to `config.toml`. This will be the toggle controlled by the user in the desktop client.</item>
                        <item name="Dynamic Job Handling">When in `sell` mode, the daemon will:
                            <ul>
                                <li>Disregard its local profit-switching engine.</li>
                                <li>Connect to the Fleet Controller and signal its availability to the matching engine.</li>
                                <li>Listen for a `START_MINING_FOR_ORDER` command via its WebSocket connection. This command will now contain the buyer's full pool details (host, port, user, pass) and the `delivery_id`.</li>
                                <li>Dynamically reconfigure and launch the correct third-party miner with the buyer's provided credentials.</li>
                                <li>If it receives a `STOP_MINING_FOR_ORDER` command (e.g., if the order is fulfilled or cancelled), it will terminate the miner and return to an idle, available state.</li>
                            </ul>
                        </item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Implement Seller UI in C++/Qt Client</summary>
                <details>
                    <sub_action name="Add a new 'Marketplace' section to the desktop client">
                        <item name="UI Controls">
                            <ul>
                                <li>Add a prominent toggle switch allowing the user to switch their `mining_mode` between "Self-Mine (Pool/Solo)" and "Sell Hashpower."</li>
                                <li>Changing this setting will send a command to the local daemon to update its configuration and restart in the new mode.</li>
                            </ul>
                        </item>
                        <item name="Seller Dashboard">
                            <ul>
                                <li>Display the rig's current seller status (e.g., "Idle - Awaiting Order," or "Fulfilling Order #123 for KawPow").</li>
                                <li>Include a new panel that makes an authenticated call to the Marketplace API to show the user's total earnings from selling hashpower, including historical charts.</li>
                            </ul>
                        </item>
                    </sub_action>
                </details>
            </action_item>
            
            <action_item>
                <summary>Implement Buyer UI on Web Dashboard</summary>
                <details>
                    <sub_action name="Add a new 'Buy Hashpower' section to the web application">
                        <item name="Order Creation Form">Build a user-friendly, multi-step form for creating new `hash_orders`. The form will allow the buyer to:
                            <ul>
                                <li>Select an algorithm from a list of those with available sellers.</li>
                                <li>Enter their own pool's connection details.</li>
                                <li>Set their price, speed limit, and total budget.</li>
                                <li>The UI will provide real-time cost estimates based on their inputs.</li>
                            </ul>
                        </item>
                        <item name="Order Management Dashboard">Create a new page where buyers can view their active and past orders. For active orders, it will display real-time hashrate delivery (via a WebSocket feed from the backend) and budget depletion, and provide a "Cancel Order" button.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Log Completion of Task 5.2</summary>
                <log_entry>
                     <validation_method>Executed a full end-to-end marketplace test. In the desktop client, switched a test rig to "Sell Hashpower" mode. On the web dashboard, logged in as a different user (buyer) and placed a new order for hashrate on the Kaspa algorithm. Verified the matching engine correctly dispatched the seller's rig, which automatically started mining to the buyer's pool. Observed the live hashrate delivery on the buyer's web dashboard and the "Fulfilling Order" status on the seller's desktop client. All validation criteria are met.</validation_method>
                     <review_outcome>Approved.</review_outcome>
                </log_entry>
                <git_commit>
                    <commands>
                        <command>git add daemon/src/marketplace_client.rs client/src/marketplace_ui.rs</command>
                        <command>git commit -m "Phase 5.2: Integrated Daemon & Client with Hashpower Marketplace."</command>
                        <command>git push origin develop</command>
                    </commands>
                    <remote_repository>https://github.com/emiliancristea/bunker-miner.git</remote_repository>
                    <branch>develop</branch>
                </git_commit>
            </action_item>
        </actions>

        <design_rationale>
            Providing distinct, purpose-built experiences for buyers (on the web) and sellers (in the native client) is a key UX decision. Buyers need the flexibility and accessibility of a web interface to manage orders from anywhere. Sellers need the tight hardware integration and real-time feedback of the native desktop client. The daemon's ability to dynamically reconfigure for a buyer's job is the core technical enabler of the entire marketplace.
        </design_rationale>

        <operational_considerations>
            <item name="Security">The daemon must treat all pool details received from the Controller as untrusted. It must validate them to the best of its ability (e.g., check for valid host formats) before using them to construct miner arguments, to prevent command injection.</item>
            <item name="User Experience">The feedback loop for both buyer and seller is critical. The UIs must provide clear, real-time status updates so that both parties have confidence that the transaction is proceeding correctly.</item>
        </operational_considerations>

        <validation_criteria>
            - A user can enable "seller mode" in the desktop client, and the daemon correctly signals its availability to the marketplace.
            - A user can create, monitor, and cancel a hashpower order from the web dashboard.
            - The full end-to-end flow is functional: a buyer's order on the web is automatically fulfilled by a seller's rig.
            - Both the seller's desktop client and the buyer's web dashboard accurately reflect the live state of the hashpower delivery.
        </validation_criteria>
        
        <testing_methodologies>
            <item name="End-to-End (E2E) System Testing">The primary validation is a full E2E test involving a buyer on the web dashboard and a seller using the desktop client, with their respective daemons and the full backend stack running in the staging environment.</item>
            <item name="UI/UX Testing">Manual testing of both the desktop and web UIs to ensure they are intuitive, clear, and provide the necessary feedback to the user.</item>
        </testing_methodologies>
        
        <version_control_strategy>
            <item name="Branching">The marketplace integration work is developed on a `feature/marketplace-integration` branch.</item>
            <item name="Commits">The Git Commit message for this task will be exactly as specified.</item>
        </version_control_strategy>
        
        <security_audit_checkpoints>
            <checkpoint>A mandatory security review of the daemon's logic for handling dynamically provided pool credentials from the controller is required. This is a critical point to prevent abuse.</checkpoint>
            <checkpoint>The web dashboard's order creation form must be reviewed for security vulnerabilities, especially ensuring that a buyer cannot drain their account or affect other users through malformed API requests.</checkpoint>
        </security_audit_checkpoints>
    </task>
</prompt>