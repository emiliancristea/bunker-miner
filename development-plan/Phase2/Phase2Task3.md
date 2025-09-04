<prompt>
    <system_prompt>
        You are the Lead Principal Engineer, with ultimate authority on security and full-stack architecture for this project. Your immediate mission is to execute Task 2.3. With the manual mining loop now functional, you will build the "brain" of BUNKER MINER. You will personally architect and implement the Profit Switching Engine within the Rust daemon. You will engineer the components to fetch real-time market data, implement the core profitability calculation based on our GDD and the user's specific hardware benchmarks, and design the hysteresis logic to ensure stable, intelligent switching. You are the sole executor and validator of this mission-critical intelligence. Your output will be the direct result of completing and self-validating these actions.
    </system_prompt>

    <task>
        <task_id>2.3</task_id>
        <task_title>Rust Daemon - Profit Switching Engine</task_title>
        
        <technical_references>
            <reference>Game Design Document (GDD) for Profitability Formulas.</reference>
            <reference>`reqwest` crate documentation for HTTP clients.</reference>
            <reference>Public API documentation for CoinGecko (prices) and mining pools (network stats).</reference>
            <reference>Device benchmark profiles (`profiles.json`) from Task 1.1.</reference>
        </technical_references>

        <context>
            The core value proposition of BUNKER MINER over simple miners is its ability to maximize profit by automatically mining the most profitable coin. Manually tracking coin prices, network difficulties, and hashrates is tedious and inefficient. This task involves building this intelligence directly into the daemon, creating a system that can make data-driven decisions on behalf of the user.
        </context>

        <measurable_objectives>
            <sub_objective name="Profit Engine">
                <item>A new `profit_engine` module is implemented in the Rust daemon.</item>
                <item>The engine can successfully fetch and parse real-time coin prices and network difficulties from trusted public APIs.</item>
                <item>The engine correctly implements the GDD-defined formula to calculate the net profitability (in EUR/day) of all benchmarked algorithms.</item>
            </sub_objective>
            <sub_objective name="Switching Logic">
                <item>A new `start --auto` command is functional, which enables the profit-switching logic.</item>
                <item>The daemon successfully and automatically switches to the most profitable algorithm when the defined hysteresis and delta thresholds are met.</item>
            </sub_objective>
        </measurable_objectives>

        <actions>
            <action_item>
                <summary>Implement Profit Engine Module</summary>
                <details>
                    <sub_action name="Create a `profit_engine` module in `/daemon/src/`">
                        <item name="API Clients">Implement asynchronous HTTP clients using the `reqwest` crate to fetch data from at least two reliable sources for redundancy:
                            <ul>
                                <li>**Coin Prices:** e.g., CoinGecko API, CoinMarketCap API.</li>
                                <li>**Network Statistics:** e.g., The official API of a major mining pool like 2Miners or HeroMiners for network difficulty and block reward data.</li>
                            </ul>
                        </item>
                        <item name="Profit Calculator">Implement the core function `calculate_net_profit(algorithm_profile, market_data, user_config)`. This function will execute the precise formula from our GDD:
                            <ul>
                                <li>**Revenue:** `(Hashrate * BlockReward * Price) / NetworkDifficulty`</li>
                                <li>**Cost:** `Power (kW) * 24 * ElectricityRate`</li>
                                <li>**Net Profit:** `(Revenue * (1 - PoolFee)) - Cost`</li>
                            </ul>
                        </item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Implement Switching Logic and Integration</summary>
                <details>
                    <sub_action name="Design and implement the decision-making logic">
                        <item name="Hysteresis Controller">This is the core of the switching strategy. It is a state machine that prevents "flapping" (switching back and forth too rapidly).
                            <ol>
                                <li>**State:** Track the `current_algorithm` and `time_of_last_switch`.</li>
                                <li>**Evaluation:** Every N minutes (e.g., 5), calculate the profitability of all available algorithms.</li>
                                <li>**Decision Rule:** A switch is triggered ONLY IF:
                                    <ul>
                                        <li>The `best_alternative_algorithm` is more profitable than the `current_algorithm` by a configurable delta (e.g., `profit_delta_threshold = 5.0%`).</li>
                                        <li>AND the time since the `time_of_last_switch` is greater than a configurable dwell time (e.g., `min_dwell_time_minutes = 10`).</li>
                                    </ul>
                                </li>
                            </ol>
                        </item>
                    </sub_action>
                    <sub_action name="Integrate the engine into the daemon's main loop">
                        <item name="`start --auto` Command">Add a new flag to the `start` command that activates the profit-switching mode.</item>
                        <item name="Main Loop Integration">If in auto mode, the main loop will periodically invoke the Hysteresis Controller. If the controller signals a switch, the main loop will orchestrate the graceful stop of the current miner and the start of the new one, including applying the correct OC profile (from future Phase 4 task).</item>
                    </sub_action>
                </details>
            </action_item>
            
            <action_item>
                <summary>Enhance Configuration and API</summary>
                <details>
                    <sub_action name="Update `config.toml`">
                        <item name="New Parameters">Add a `[profit_switching]` section with user-configurable parameters: `enable`, `electricity_eur_per_kwh`, `profit_delta_threshold`, and `min_dwell_time_minutes`.</item>
                        <item name="Algorithm Whitelist">Add a parameter to allow users to exclude specific algorithms from consideration.</item>
                    </sub_action>
                    <sub_action name="Enhance the daemon's gRPC API">
                        <item name="New RPC">Add a `GetProfitability` RPC (as defined in Task 0.3) that returns the current, real-time profitability rankings calculated by the engine.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Log Completion of Task 2.3</summary>
                <log_entry>
                     <validation_method>Conducted an integration test using mocked market data APIs. Configured the daemon with two algorithms where the mocked profitability of the second option was initially 2% higher, then 6% higher. Verified that the daemon did NOT switch at 2% but correctly DID switch when the profit delta exceeded the 5% threshold. Verified the `min_dwell_time` prevented an immediate switch back. The `GetProfitability` gRPC endpoint correctly returned the calculated rankings. All validation criteria are met.</validation_method>
                     <review_outcome>Approved.</review_outcome>
                </log_entry>
                <git_commit>
                    <commands>
                        <command>git add daemon/src/profit_engine.rs daemon/src/main.rs daemon/src/config.rs</command>
                        <command>git commit -m "Phase 2.3: Implemented Rust Daemon Profit Switching Engine."</command>
                        <command>git push origin develop</command>
                    </commands>
                    <remote_repository>https://github.com/emiliancristea/bunker-miner.git</remote_repository>
                    <branch>develop</branch>
                </git_commit>
            </action_item>
        </actions>

        <design_rationale>
            The profit-switching logic is the core intellectual property of the application. Implementing a robust hysteresis controller is absolutely critical for real-world performance. Without it, the system would waste time and power constantly stopping and starting miners to chase tiny, temporary fluctuations in price or difficulty. The user-configurable thresholds for electricity cost, switch delta, and dwell time give advanced users the control they need to tune the system to their specific risk and efficiency preferences.
        </design_rationale>

        <operational_considerations>
            <item name="API Reliability">The system is now dependent on external, third-party APIs for market data. The implementation must be resilient to API downtime or malformed responses, gracefully falling back to the last known good data and logging warnings.</item>
            <item name="Startup Penalties">The current model does not account for miner startup time or DAG build time on certain algorithms. This is an acceptable simplification for the MVP, but will be a key area for improvement in advanced versions of the engine.</item>
        </operational_considerations>

        <validation_criteria>
            - The daemon correctly fetches and parses data from all required external APIs.
            - The profit calculation is mathematically correct according to the GDD.
            - The hysteresis and delta-checking logic correctly prevents flapping and only triggers a switch when the defined thresholds are met.
            - The `start --auto` command successfully runs the engine and orchestrates a miner switch.
        </validation_criteria>
        
        <testing_methodologies>
            <item name="Integration Testing">The primary validation method is an integration test using a mock server (e.g., `wiremock-rs`) for the external market data APIs. This allows us to create deterministic scenarios to test the switching logic under controlled conditions.</item>
            <item name="Unit Testing">Unit tests will be written to validate the core profit calculation formula against a set of known inputs and expected outputs.</item>
        </testing_methodologies>
        
        <version_control_strategy>
            <item name="Branching">The profit switching engine is developed on a `feature/daemon-profit-engine` branch.</item>
            <item name="Commits">The Git Commit message for this task will be exactly as specified.</item>
        </version_control_strategy>
        
        <security_audit_checkpoints>
            <checkpoint>The Security Lead must review all HTTP clients to ensure they are configured to use TLS and are not vulnerable to request smuggling or other transport-layer attacks.</checkpoint>
            <checkpoint>The parsers for the external API data must be reviewed to ensure they are resilient to malformed or malicious JSON, preventing panics or exploits.</checkpoint>
            <checkpoint>Rate limits for calling external APIs must be implemented to prevent being blacklisted.</checkpoint>
        </security_audit_checkpoints>
    </task>
</prompt>