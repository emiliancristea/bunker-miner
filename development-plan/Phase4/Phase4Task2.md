<prompt>
    <system_prompt>
        You are the Lead Principal Engineer, with ultimate authority on security and full-stack architecture for this project. Your immediate mission is to execute Task 4.2. With the daemon now highly intelligent, you will build the centralized "command center" for professional users. You will personally architect and implement the backend services and the web frontend for the BUNKER MINER Fleet Management system. You will build a secure web application with user authentication, a robust database schema for managing rigs, and a high-performance, secure WebSocket-based Controller API for the daemons to connect to. You are the sole architect and validator of this new, critical piece of backend infrastructure. Your output will be the direct result of completing and self-validating these actions.
    </system_prompt>

    <task>
        <task_id>4.2</task_id>
        <task_title>Fleet Management - Web Dashboard & Controller API</task_title>
        
        <technical_references>
            <reference>Finalized ADR for Fleet Management Architecture.</reference>
            <reference>`axum` and `tokio-tungstenite` crate documentation.</reference>
            <reference>PostgreSQL and `sqlx` crate documentation.</reference>
            <reference>React/Vue/Svelte and modern web development best practices.</reference>
        </technical_references>

        <context>
            Managing more than one rig with a desktop client is inefficient. Professional and multi-rig miners need a centralized web dashboard to monitor and control their entire fleet from anywhere. This task involves building the backend services (user accounts, database, and a real-time daemon API) and the web frontend (dashboard) for this fleet management system. This transforms BUNKER MINER from a single-instance application into a scalable, multi-rig platform.
        </context>

        <measurable_objectives>
            <sub_objective name="Backend Infrastructure">
                <item>A new, secure web application backend (BUNKER MINER Web) is implemented and deployed to the staging environment.</item>
                <item>The backend supports secure user account registration and login via JWTs.</item>
                <item>A secure WebSocket-based Controller API is functional, allowing daemons to authenticate and connect.</item>
            </sub_objective>
            <sub_objective name="Frontend UI">
                <item>A web dashboard is created that allows a logged-in user to view a list of their registered rigs.</item>
                <item>The dashboard displays aggregated and per-rig telemetry received from connected daemons in real time.</item>
            </sub_objective>
        </measurable_objectives>

        <actions>
            <action_item>
                <summary>Implement Fleet Management Backend Service</summary>
                <details>
                    <sub_action name="Develop the Fleet Controller as a new service (or enhance the existing Pool API service)">
                        <item name="Web Authentication">Implement a standard, secure web authentication system. This includes user registration (email/password), password hashing (`argon2` or `bcrypt`), and session management using secure, short-lived JWTs.</item>
                        <item name="Database Schema">Extend the PostgreSQL database with new tables:
                            <ul>
                                <li>`users`: `user_id`, `email`, `password_hash`.</li>
                                <li>`rigs`: `rig_id`, `owner_user_id`, `rig_name`, `status` (online/offline).</li>
                                <li>`api_keys`: `key_id`, `owner_user_id`, `key_hash`, `prefix`, `created_at`.</li>
                            </ul>
                        </item>
                        <item name="Controller WebSocket API">
                            <ul>
                                <li>Implement a `/fleet/ws` WebSocket endpoint using `axum` and `tokio-tungstenite`.</li>
                                <li>**Authentication (Critical Security):** When a daemon attempts to connect, it must provide a long-lived API key. The server will hash the provided key and compare it to the stored `key_hash` in the database to authenticate the rig.</li>
                                <li>The server will maintain a map of `rig_id` to the active WebSocket connection to route commands.</li>
                                <li>It will receive telemetry from the daemon and broadcast it to the appropriate web dashboard clients.</li>
                            </ul>
                        </item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Develop Web Dashboard Frontend</summary>
                <details>
                    <sub_action name="Create a new single-page application (SPA) project">
                        <item name="Framework">Choose and set up a modern web framework like React, Vue, or Svelte.</item>
                        <item name="Authentication UI">Build the login and registration pages. After a successful login, the JWT must be securely stored in the browser (e.g., in a secure, HttpOnly cookie).</item>
                        <item name="Dashboard View">
                            <ul>
                                <li>This is the main view after login. It will make an authenticated REST API call to fetch the user's list of rigs.</li>
                                <li>It will establish a WebSocket connection to the backend to receive real-time updates.</li>
                                <li>It will display a list of all registered rigs with their key telemetry: status (online/offline), current algorithm, total hashrate, temperature, and profitability.</li>
                            </ul>
                        </item>
                        <item name="Rig Detail View">Create a drill-down page that shows historical charts and detailed, per-device telemetry for a single rig.</item>
                    </sub_action>
                </details>
            </action_item>
            
            <action_item>
                <summary>Containerize and Deploy to Staging</summary>
                <details>
                    <sub_action name="Create production-grade Dockerfiles and Kubernetes manifests">
                        <item name="Containerization">Create a multi-stage Dockerfile for the web frontend (build step + Nginx serving step) and another for the backend controller service.</item>
                        <item name="Deployment">Deploy the services to the staging EKS cluster behind the existing Ingress controller.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Log Completion of Task 4.2</summary>
                <log_entry>
                     <validation_method>Successfully deployed the new Fleet Management backend and web frontend to the staging environment. Created a new user account via the web UI. Generated an API key. Configured a test daemon instance with this key. Verified the daemon successfully authenticated and connected to the Controller's WebSocket API. The rig appeared on the web dashboard, and its live telemetry was displayed correctly and updated in real time. All validation criteria are met.</validation_method>
                     <review_outcome>Approved.</review_outcome>
                </log_entry>
                <git_commit>
                    <commands>
                        <command>git add pool/src/fleet_controller.rs web/dashboard/ web/api/</command>
                        <command>git commit -m "Phase 4.2: Implemented Fleet Management Web Dashboard & Controller API."</command>
                        <command>git push origin develop</command>
                    </commands>
                    <remote_repository>https://github.com/emiliancristea/bunker-miner.git</remote_repository>
                    <branch>develop</branch>
                </git_commit>
            </action_item>
        </actions>

        <design_rationale>
            The Controller/Agent model using a persistent WebSocket is a highly efficient and scalable pattern for real-time fleet management. It allows the backend to push commands to the daemons instantly, rather than relying on inefficient polling. A separate, long-lived API key for daemons is a critical security pattern that decouples machine authentication from user authentication, and allows keys to be revoked without forcing the user to change their password.
        </design_rationale>

        <operational_considerations>
            <item name="Scalability">The Fleet Controller is a stateful service (it holds open WebSocket connections). It must be designed for horizontal scaling using a Redis backplane to share connection state across multiple instances, ensuring a user's web session and their rig's connection are handled correctly regardless of which pod they connect to.</item>
            <item name="Data Security">All communication between the daemon and the controller, and between the user's browser and the web dashboard, must be over secure, encrypted channels (WSS and HTTPS).</item>
        </operational_considerations>

        <validation_criteria>
            - A user can successfully create an account on the web dashboard and generate a rig API key.
            - A daemon configured with the API key successfully connects and authenticates to the Controller API.
            - The web dashboard accurately displays the live status and telemetry of all connected rigs for a logged-in user.
            - The entire stack is successfully deployed and functional in the staging environment.
        </validation_criteria>
        
        <testing_methodologies>
            <item name="API Testing">Using tools like Postman or a custom test script to test the web authentication and REST API endpoints.</item>
            <item name="Integration Testing">The primary validation is a full integration test: a live daemon connecting to the live backend, and a live web frontend connecting to the same backend to verify the data flows correctly end-to-end.</item>
            <item name="UI/UX Testing">Manual testing of the web dashboard for usability, clarity, and responsiveness.</item>
        </testing_methodologies>
        
        <version_control_strategy>
            <item name="Branching">The fleet management backend and frontend are developed on a `feature/fleet-management` branch.</item>
            <item name="Commits">The Git Commit message for this task will be exactly as specified.</item>
        </version_control_strategy>
        
        <security_audit_checkpoints>
            <checkpoint>A mandatory, exhaustive security review of the web authentication system is required. This must cover password hashing, JWT generation and validation, and protection against common web vulnerabilities (XSS, CSRF, SQLi).</checkpoint>
            <checkpoint>The daemon API key authentication mechanism must be reviewed to ensure it is secure against timing and replay attacks. API keys must be stored as hashes in the database, never in plaintext.</checkpoint>
            <checkpoint>The WebSocket server must be reviewed for security, particularly its authentication and message handling logic, to prevent unauthorized access or denial-of-service.</checkpoint>
        </security_audit_checkpoints>
    </task>
</prompt>