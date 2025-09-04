<prompt>
    <system_prompt>
        You are the Lead Principal Engineer, with ultimate authority on security and full-stack architecture for this project. Your immediate mission is to execute Task 5.3. With the core ecosystem now built, you will ensure its longevity and adaptability. You will personally architect and implement a secure Plugin SDK that allows the community to extend BUNKER MINER's capabilities. Your top priority is security. You will use WebAssembly (WASM) to create a strictly sandboxed environment, ensuring that third-party plugins can have no access to the user's system beyond what is explicitly permitted. You will refactor the daemon to dynamically load these plugins, create a public template for developers, and validate the entire system by migrating an existing miner adapter into the new plugin format. You are the sole executor and validator of this critical, security-focused architecture. Your output will be the direct result of completing and self-validating these actions.
    </system_prompt>

    <task>
        <task_id>5.3</task_id>
        <task_title>Plugin SDK & Community Miner Integration</task_title>
        
        <technical_references>
            <reference>Finalized ADR for Plugin SDK Architecture.</reference>
            <reference>WebAssembly (WASM) and WASI (WebAssembly System Interface) specifications.</reference>
            <reference>`wasmer` or `wasmtime` crate documentation for sandboxed WASM execution.</reference>
        </technical_references>

        <context>
            The crypto mining landscape evolves rapidly, with new miners and algorithms appearing constantly. To keep BUNKER MINER at the cutting edge and reduce the development bottleneck on our core team, we must empower the community to extend its capabilities. This task involves designing and implementing a secure Plugin SDK that allows third-party developers to add support for new miners without requiring a full application update. Security is the paramount concern, as we will be running untrusted, community-provided code.
        </context>

        <measurable_objectives>
            <sub_objective name="SDK & Architecture">
                <item>A formal Plugin SDK is designed and documented, using WebAssembly (WASM) as the core technology.</item>
                <item>The Rust daemon is refactored to dynamically and securely load, sandbox, and interact with `.wasm` plugin files.</item>
            </sub_objective>
            <sub_objective name="Validation">
                <item>A public "template" plugin repository is created to guide community developers.</item>
                <item>An existing, hardcoded miner adapter is successfully migrated to the new plugin system, validating the entire architecture end-to-end.</item>
            </sub_objective>
            <sub_objective name="Security">
                <item>The plugin system runs in a secure, restrictive sandbox that is proven to prevent unauthorized filesystem and network access.</item>
            </sub_objective>
        </measurable_objectives>

        <actions>
            <action_item>
                <summary>Design and Implement the Secure WASM-based Plugin SDK</summary>
                <details>
                    <sub_action name="Finalize the Plugin SDK architecture as per the ADR">
                        <item name="Technology">Standardize on WebAssembly (WASM) as the plugin binary format. This provides a language-agnostic and, most importantly, inherently sandboxed execution environment.</item>
                        <item name="Host Runtime">Integrate a mature WASM runtime like `wasmer` or `wasmtime` into the Rust daemon. This runtime will be responsible for loading and executing the plugin code.</item>
                        <item name="Security Sandbox (Critical)">
                            <ul>
                                <li>The WASM runtime must be configured with the most restrictive permissions possible.</li>
                                <li>By default, a plugin will have **no access** to the filesystem, network, or any system resources (WASI capabilities will be disabled).</li>
                                <li>The daemon (the "host") will expose a very small, specific set of functions to the plugin (the "guest"), such as `log_message()` or `get_config_value()`. The plugin cannot do anything the host does not explicitly allow.</li>
                            </ul>
                        </item>
                        <item name="Plugin Interface">Define the canonical interface that every miner plugin `.wasm` file must export. This will be a set of functions that mirrors the existing `MinerAdapter` trait, such as `fn build_args(config_json: &str) -> String` and `fn parse_output(log_line: &str) -> Option<TelemetryJson>`. Communication will happen via JSON-serialized strings over this simple interface.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Refactor Daemon to be Plugin-Driven</summary>
                <details>
                    <sub_action name="Update the daemon's `miners` module">
                        <item name="Dynamic Loading">At startup, the daemon will scan a `/plugins` subdirectory for any `.wasm` files.</item>
                        <item name="Plugin Instantiation">For each valid `.wasm` file found, the daemon will load it into its own sandboxed `wasmer` instance.</item>
                        <item name="Unified Adapter">The internal `MinerAdapter` trait will now be a wrapper. It will check if the requested miner corresponds to a loaded WASM plugin or a built-in, hardcoded adapter. This allows for a gradual migration and maintains backward compatibility.</item>
                    </sub_action>
                </details>
            </action_item>
            
            <action_item>
                <summary>Create Developer Tooling and Validate the System</summary>
                <details>
                    <sub_action name="Create a public 'BunkerMiner-Plugin-Template' GitHub repository">
                        <item name="Template Code">Provide a minimal, working example of a miner plugin written in Rust that compiles to WASM. This will be the starting point for all community developers.</item>
                        <item name="Documentation">Write clear, comprehensive documentation for the Plugin SDK, explaining the interface, the security sandbox constraints, and the process for building and testing a plugin.</item>
                    </sub_action>
                    <sub_action name="Validate the SDK by migrating an existing miner">
                        <item name="Migration">Take the existing, hardcoded logic for the GMiner adapter and re-implement it in a new, separate Rust project that compiles to `gminer.wasm`.</item>
                        <item name="End-to-End Test">Remove the hardcoded GMiner adapter from the daemon. Place the new `gminer.wasm` file in the `/plugins` directory. Run the daemon and start mining with GMiner. The entire flow must work exactly as it did before, proving the plugin system is a viable replacement.</item>
                    </sub_action>
                </details>
            </action_item>

            <action_item>
                <summary>Log Completion of Task 5.3</summary>
                <log_entry>
                     <validation_method>Successfully refactored the daemon to support the WASM plugin architecture. Created the public plugin template repository. Migrated the existing GMiner adapter into a `gminer.wasm` plugin. With the hardcoded adapter removed, the daemon successfully loaded the WASM plugin and initiated mining with GMiner, proving the system works end-to-end. A malicious test plugin attempting to access the filesystem was correctly blocked by the WASM sandbox, triggering a security violation log. All validation criteria are met.</validation_method>
                     <review_outcome>Approved.</review_outcome>
                </log_entry>
                <git_commit>
                    <commands>
                        <command>git add daemon/src/plugin_sdk.rs daemon/src/sandbox.rs daemon/src/wasm_runtime.rs</command>
                        <command>git commit -m "Phase 5.3: Implemented Secure WASM-based Plugin SDK for Community Miners."</command>
                        <command>git push origin develop</command>
                    </commands>
                    <remote_repository>https://github.com/emiliancristea/bunker-miner.git</remote_repository>
                    <branch>develop</branch>
                </git_commit>
            </action_item>
        </actions>

        <design_rationale>
            Using WebAssembly is the modern, secure solution for building a plugin architecture. Unlike older, more dangerous methods (like loading dynamic-link libraries/.so files), WASM provides a mathematically provable sandbox by default. It is impossible for a WASM plugin to perform a malicious action unless we explicitly provide it with the capability to do so. This "default-deny" security posture is non-negotiable for a system that will run untrusted, community-provided code.
        </design_rationale>

        <operational_considerations>
            <item name="Plugin Vetting">While the sandbox provides technical safety, we will still need a community process for reviewing and "verifying" plugins. The UI will clearly distinguish between "Official" and "Community" plugins.</item>
            <item name="Performance">There is a minor performance overhead to calling functions across the WASM host/guest boundary. This is negligible for the tasks a miner plugin performs (config parsing, regex), but it must be benchmarked.</item>
            <item name="SDK Versioning">The Plugin SDK interface must be versioned. The daemon will need to check the version of a plugin it loads to ensure compatibility.</item>
        </operational_considerations>

        <validation_criteria>
            - The daemon successfully loads and runs a `.wasm` miner plugin.
            - An existing miner adapter, when migrated to a plugin, provides the exact same functionality as the hardcoded version.
            - A malicious test plugin attempting to access the filesystem or network is correctly and verifiably blocked by the WASM sandbox.
            - The public plugin template and its documentation are complete and published.
        </validation_criteria>
        
        <testing_methodologies>
            <item name="Security Testing">The primary validation method for the sandbox is to write and test a malicious plugin that actively tries to perform forbidden actions (file I/O, network calls) and verify that the host runtime traps and denies these attempts.</item>
            <item name="Integration Testing">Validating the full lifecycle of the migrated GMiner plugin to ensure it performs identically to its hardcoded predecessor.</item>
        </testing_methodologies>
        
        <version_control_strategy>
            <item name="Branching">The plugin SDK and daemon refactoring are developed on a `feature/plugin-sdk` branch.</item>
            <item name="Commits">The Git Commit message for this task will be exactly as specified.</item>
        </version_control_strategy>
        
        <security_audit_checkpoints>
            <checkpoint>A mandatory, exhaustive security review of the WASM plugin architecture is required. This is the most critical security review of Phase 5. The review must focus on the configuration of the sandbox runtime to ensure it is maximally restrictive and that no unsafe capabilities are exposed to plugins.</checkpoint>
            <checkpoint>The interface between the host and the plugin must be reviewed to ensure it is not vulnerable to abuse (e.g., passing malicious data that could exploit the host's logic).</checkpoint>
        </security_audit_checkpoints>
    </task>
</prompt>