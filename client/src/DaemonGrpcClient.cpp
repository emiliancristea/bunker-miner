#include "DaemonGrpcClient.h"
#include "TelemetryWorker.h"
#include <QDebug>
#include <QTimer>
#include <QDateTime>
#include <QThread>
#include <google/protobuf/empty.pb.h>

/**
 * DaemonGrpcClient implementation - Phase 2.2
 * 
 * Enhanced gRPC client implementation for BUNKER MINER daemon communication
 * with real-time telemetry streaming and mining control capabilities.
 * 
 * Phase 2.2 Features:
 * - Real-time telemetry streaming with TelemetryWorker thread management
 * - Complete mining control operations (start/stop with state validation)
 * - Thread-safe telemetry data relay to UI components
 * - Mining state tracking and error handling
 * - Secure localhost-only connection by default
 * - Comprehensive error handling and connection retry logic
 * - Health monitoring with automatic reconnection
 */

DaemonGrpcClient::DaemonGrpcClient(QObject *parent)
    : QObject(parent)
    , m_connected(false)
    , m_daemonAddress("127.0.0.1:50051")
    , m_grpcChannel(nullptr)
    , m_grpcStub(nullptr)
    , m_healthCheckTimer(new QTimer(this))
    , m_reconnectTimer(new QTimer(this))
    , m_connectionRetryCount(0)
    , m_miningActive(false)
    , m_telemetryWorker(nullptr)
    , m_telemetryThread(nullptr)
    , m_telemetryStreamActive(false)
{
    qDebug() << "DaemonGrpcClient initialized for BUNKER MINER daemon communication with telemetry streaming";
    
    initializeMiningState();
    
    // Configure health check timer
    m_healthCheckTimer->setSingleShot(false);
    m_healthCheckTimer->setInterval(HEALTH_CHECK_INTERVAL_MS);
    connect(m_healthCheckTimer, &QTimer::timeout, this, &DaemonGrpcClient::checkConnectionHealth);
    
    // Configure reconnect timer
    m_reconnectTimer->setSingleShot(true);
    m_reconnectTimer->setInterval(RECONNECT_INTERVAL_MS);
    connect(m_reconnectTimer, &QTimer::timeout, [this]() {
        if (m_connectionRetryCount < MAX_RETRY_COUNT) {
            qDebug() << "Attempting automatic reconnection, retry" << m_connectionRetryCount + 1;
            connectToDaemon(m_daemonAddress);
        }
    });
}

DaemonGrpcClient::~DaemonGrpcClient() {
    // Stop telemetry streaming first
    stopTelemetryStream();
    cleanupTelemetryWorker();
    
    // Stop mining if active
    if (m_miningActive.load()) {
        stopMiningOperation();
    }
    
    cleanupGrpcClient();
}

bool DaemonGrpcClient::connectToDaemon(const QString &address) {
    // Security check: Only allow localhost connections by default
    if (!address.startsWith("127.0.0.1:") && !address.startsWith("localhost:")) {
        qWarning() << "Security: Only localhost connections allowed by default. Address:" << address;
        emit connectionError("Security: Remote connections not permitted without explicit configuration");
        return false;
    }
    
    m_daemonAddress = address;
    qDebug() << "Attempting to connect to BUNKER MINER daemon at:" << address;
    
    try {
        initializeGrpcClient();
        
        // Perform initial health check to verify connection
        if (performHealthCheck()) {
            m_connected = true;
            m_connectionRetryCount = 0;
            m_healthCheckTimer->start();
            
            emit connected();
            qDebug() << "Successfully connected to daemon at" << address;
            return true;
        } else {
            // Connection failed, schedule retry if under limit
            m_connectionRetryCount++;
            if (m_connectionRetryCount < MAX_RETRY_COUNT) {
                qDebug() << "Connection failed, scheduling retry" << m_connectionRetryCount;
                m_reconnectTimer->start();
            } else {
                qWarning() << "Maximum connection retries reached";
                emit connectionError("Connection failed after maximum retries. Is the daemon running?");
            }
            return false;
        }
    } catch (const std::exception &e) {
        qWarning() << "gRPC connection exception:" << e.what();
        emit connectionError(QString("Connection error: %1").arg(e.what()));
        return false;
    }
}

void DaemonGrpcClient::disconnectFromDaemon() {
    if (m_connected) {
        m_connected = false;
        m_healthCheckTimer->stop();
        m_reconnectTimer->stop();
        
        cleanupGrpcClient();
        
        emit disconnected();
        qDebug() << "Disconnected from daemon";
    }
}

bool DaemonGrpcClient::isConnected() const {
    return m_connected;
}

void DaemonGrpcClient::getSystemInfo() {
    if (!m_connected || !m_grpcStub) {
        emit connectionError("Not connected to daemon");
        return;
    }
    
    try {
        ClientContext context;
        google::protobuf::Empty request;
        bunker::daemon::v1::SystemInfoResponse response;
        
        // Set timeout for the call
        context.set_deadline(std::chrono::system_clock::now() + std::chrono::seconds(10));
        
        Status status = m_grpcStub->GetSystemInfo(&context, request, &response);
        
        if (status.ok()) {
            qDebug() << "Successfully received system information";
            
            // Convert protobuf data to Qt structures
            std::vector<DeviceInfo> devices;
            for (const auto &pbDevice : response.devices()) {
                devices.push_back(convertDeviceInfo(pbDevice));
            }
            
            SystemInfo systemInfo = convertSystemInfo(response.system_info());
            VersionInfo versionInfo = convertVersionInfo(response.version_info());
            
            emit systemInfoReceived(devices, systemInfo, versionInfo);
        } else {
            QString errorMsg = QString("GetSystemInfo failed: %1 (%2)")
                              .arg(QString::fromStdString(status.error_message()))
                              .arg(status.error_code());
            qWarning() << errorMsg;
            emit connectionError(errorMsg);
        }
    } catch (const std::exception &e) {
        QString errorMsg = QString("System info request exception: %1").arg(e.what());
        qWarning() << errorMsg;
        emit connectionError(errorMsg);
    }
}

void DaemonGrpcClient::getHealthCheck() {
    if (!m_connected || !m_grpcStub) {
        emit connectionError("Not connected to daemon");
        return;
    }
    
    try {
        ClientContext context;
        google::protobuf::Empty request;
        bunker::daemon::v1::HealthCheckResponse response;
        
        context.set_deadline(std::chrono::system_clock::now() + std::chrono::seconds(5));
        
        Status status = m_grpcStub->HealthCheck(&context, request, &response);
        
        if (status.ok()) {
            QString healthStatus;
            switch (response.status()) {
                case bunker::daemon::v1::HealthCheckResponse::HEALTH_HEALTHY:
                    healthStatus = "Healthy";
                    break;
                case bunker::daemon::v1::HealthCheckResponse::HEALTH_DEGRADED:
                    healthStatus = "Degraded";
                    break;
                case bunker::daemon::v1::HealthCheckResponse::HEALTH_UNHEALTHY:
                    healthStatus = "Unhealthy";
                    break;
                default:
                    healthStatus = "Unknown";
                    break;
            }
            
            QStringList componentHealth;
            for (const auto &component : response.component_health()) {
                QString componentStatus = QString("%1: %2 - %3")
                    .arg(QString::fromStdString(component.component_name()))
                    .arg(component.status() == bunker::daemon::v1::HealthCheckResponse::HEALTH_HEALTHY ? "Healthy" : "Unhealthy")
                    .arg(QString::fromStdString(component.status_message()));
                componentHealth.append(componentStatus);
            }
            
            emit healthCheckReceived(healthStatus, componentHealth);
            qDebug() << "Health check completed, status:" << healthStatus;
        } else {
            QString errorMsg = QString("Health check failed: %1").arg(QString::fromStdString(status.error_message()));
            qWarning() << errorMsg;
            emit connectionError(errorMsg);
        }
    } catch (const std::exception &e) {
        QString errorMsg = QString("Health check exception: %1").arg(e.what());
        qWarning() << errorMsg;
        emit connectionError(errorMsg);
    }
}

void DaemonGrpcClient::startMining(const QString &algorithm) {
    if (!m_connected || !m_grpcStub) {
        emit connectionError("Not connected to daemon");
        return;
    }
    
    // Note: For Phase 2.1, this provides the framework for mining control
    // Full mining implementation will be completed in Phase 2.2
    qDebug() << "Start mining requested for algorithm:" << algorithm;
    emit commandResponse(QString("Mining start request received (Phase 2.1 - framework ready for Phase 2.2 implementation)"));
}

void DaemonGrpcClient::stopMining() {
    if (!m_connected || !m_grpcStub) {
        emit connectionError("Not connected to daemon");
        return;
    }
    
    // Note: For Phase 2.1, this provides the framework for mining control
    qDebug() << "Stop mining requested";
    emit commandResponse(QString("Mining stop request received (Phase 2.1 - framework ready for Phase 2.2 implementation)"));
}

void DaemonGrpcClient::getConfiguration() {
    if (!m_connected || !m_grpcStub) {
        emit connectionError("Not connected to daemon");
        return;
    }
    
    try {
        ClientContext context;
        bunker::daemon::v1::GetConfigRequest request;
        bunker::daemon::v1::GetConfigResponse response;
        
        context.set_deadline(std::chrono::system_clock::now() + std::chrono::seconds(10));
        
        Status status = m_grpcStub->GetConfig(&context, request, &response);
        
        if (status.ok()) {
            QString configJson = QString::fromStdString(response.config_json());
            emit configurationReceived(configJson);
            qDebug() << "Configuration retrieved successfully";
        } else {
            QString errorMsg = QString("Get configuration failed: %1").arg(QString::fromStdString(status.error_message()));
            qWarning() << errorMsg;
            emit connectionError(errorMsg);
        }
    } catch (const std::exception &e) {
        QString errorMsg = QString("Get configuration exception: %1").arg(e.what());
        qWarning() << errorMsg;
        emit connectionError(errorMsg);
    }
}

void DaemonGrpcClient::setConfiguration(const QString &configJson) {
    if (!m_connected || !m_grpcStub) {
        emit connectionError("Not connected to daemon");
        return;
    }
    
    try {
        ClientContext context;
        bunker::daemon::v1::SetConfigRequest request;
        bunker::daemon::v1::SetConfigResponse response;
        
        request.set_config_json(configJson.toStdString());
        request.set_validate_only(false);
        request.set_restart_services(true);
        
        context.set_deadline(std::chrono::system_clock::now() + std::chrono::seconds(30));
        
        Status status = m_grpcStub->SetConfig(&context, request, &response);
        
        if (status.ok()) {
            if (response.status() == bunker::daemon::v1::CommandResponse::STATUS_SUCCESS) {
                emit commandResponse("Configuration updated successfully");
                qDebug() << "Configuration updated successfully";
            } else {
                QString errorMsg = "Configuration validation failed";
                if (!response.validation_errors().empty()) {
                    errorMsg += ": ";
                    for (const auto &error : response.validation_errors()) {
                        errorMsg += QString::fromStdString(error) + "; ";
                    }
                }
                emit connectionError(errorMsg);
            }
        } else {
            QString errorMsg = QString("Set configuration failed: %1").arg(QString::fromStdString(status.error_message()));
            qWarning() << errorMsg;
            emit connectionError(errorMsg);
        }
    } catch (const std::exception &e) {
        QString errorMsg = QString("Set configuration exception: %1").arg(e.what());
        qWarning() << errorMsg;
        emit connectionError(errorMsg);
    }
}

void DaemonGrpcClient::checkConnectionHealth() {
    if (m_connected && performHealthCheck()) {
        // Connection is healthy, continue monitoring
        return;
    } else {
        // Connection lost, attempt reconnection
        qWarning() << "Connection health check failed, attempting reconnection";
        m_connected = false;
        m_connectionRetryCount = 0;
        connectToDaemon(m_daemonAddress);
    }
}

void DaemonGrpcClient::initializeGrpcClient() {
    // Create gRPC channel with security settings
    grpc::ChannelArguments channelArgs;
    channelArgs.SetMaxReceiveMessageSize(4 * 1024 * 1024); // 4MB max message size
    channelArgs.SetMaxSendMessageSize(1 * 1024 * 1024);    // 1MB max send size
    channelArgs.SetKeepAliveTime(30 * 1000);               // 30 seconds
    channelArgs.SetKeepAliveTimeout(5 * 1000);             // 5 seconds timeout
    channelArgs.SetKeepAlivePermitWithoutCalls(true);
    
    // Use insecure channel for localhost (as per daemon's security model)
    m_grpcChannel = grpc::CreateCustomChannel(m_daemonAddress.toStdString(), 
                                            grpc::InsecureChannelCredentials(),
                                            channelArgs);
    
    if (!m_grpcChannel) {
        throw std::runtime_error("Failed to create gRPC channel");
    }
    
    // Create the daemon stub
    m_grpcStub = bunker::daemon::v1::BunkerMinerDaemon::NewStub(m_grpcChannel);
    
    if (!m_grpcStub) {
        throw std::runtime_error("Failed to create gRPC stub");
    }
    
    qDebug() << "gRPC client initialized successfully";
}

void DaemonGrpcClient::cleanupGrpcClient() {
    m_grpcStub.reset();
    m_grpcChannel.reset();
    qDebug() << "gRPC client resources cleaned up";
}

bool DaemonGrpcClient::performHealthCheck() {
    if (!m_grpcStub) {
        return false;
    }
    
    try {
        ClientContext context;
        google::protobuf::Empty request;
        bunker::daemon::v1::HealthCheckResponse response;
        
        // Short timeout for health checks
        context.set_deadline(std::chrono::system_clock::now() + std::chrono::seconds(3));
        
        Status status = m_grpcStub->HealthCheck(&context, request, &response);
        
        if (status.ok()) {
            return response.status() == bunker::daemon::v1::HealthCheckResponse::HEALTH_HEALTHY ||
                   response.status() == bunker::daemon::v1::HealthCheckResponse::HEALTH_DEGRADED;
        } else {
            qDebug() << "Health check failed:" << QString::fromStdString(status.error_message());
            return false;
        }
    } catch (const std::exception &e) {
        qDebug() << "Health check exception:" << e.what();
        return false;
    }
}

DaemonGrpcClient::DeviceInfo DaemonGrpcClient::convertDeviceInfo(const bunker::daemon::v1::DeviceInfo &pbDevice) {
    DeviceInfo device;
    device.deviceId = QString::fromStdString(pbDevice.device_id());
    device.name = QString::fromStdString(pbDevice.name());
    
    // Convert vendor enum
    switch (pbDevice.vendor()) {
        case bunker::daemon::v1::DeviceInfo::VENDOR_NVIDIA:
            device.vendor = "NVIDIA";
            break;
        case bunker::daemon::v1::DeviceInfo::VENDOR_AMD:
            device.vendor = "AMD";
            break;
        case bunker::daemon::v1::DeviceInfo::VENDOR_INTEL:
            device.vendor = "Intel";
            break;
        default:
            device.vendor = "Unknown";
            break;
    }
    
    // Convert device type enum
    switch (pbDevice.device_type()) {
        case bunker::daemon::v1::DeviceInfo::DEVICE_TYPE_GPU:
            device.deviceType = "GPU";
            break;
        case bunker::daemon::v1::DeviceInfo::DEVICE_TYPE_CPU:
            device.deviceType = "CPU";
            break;
        case bunker::daemon::v1::DeviceInfo::DEVICE_TYPE_ASIC:
            device.deviceType = "ASIC";
            break;
        case bunker::daemon::v1::DeviceInfo::DEVICE_TYPE_FPGA:
            device.deviceType = "FPGA";
            break;
        default:
            device.deviceType = "Unknown";
            break;
    }
    
    device.vramMb = pbDevice.vram_mb();
    device.coreCount = pbDevice.core_count();
    device.driverVersion = QString::fromStdString(pbDevice.driver_version());
    device.computeCapability = QString::fromStdString(pbDevice.compute_capability());
    device.baseClockMhz = pbDevice.base_clock_mhz();
    device.memoryClockMhz = pbDevice.memory_clock_mhz();
    device.powerLimitWatts = pbDevice.power_limit_watts();
    
    // Convert capabilities
    for (const auto &capability : pbDevice.capabilities()) {
        device.capabilities.append(QString::fromStdString(capability));
    }
    
    return device;
}

DaemonGrpcClient::SystemInfo DaemonGrpcClient::convertSystemInfo(const bunker::daemon::v1::SystemInfoResponse_SystemInfo &pbSystem) {
    SystemInfo system;
    system.osName = QString::fromStdString(pbSystem.os_name());
    system.osVersion = QString::fromStdString(pbSystem.os_version());
    system.totalMemoryGb = pbSystem.total_memory_gb();
    system.availableMemoryGb = pbSystem.available_memory_gb();
    system.cpuName = QString::fromStdString(pbSystem.cpu_name());
    system.cpuCores = pbSystem.cpu_cores();
    system.cpuThreads = pbSystem.cpu_threads();
    system.uptimeSeconds = pbSystem.uptime_seconds();
    
    return system;
}

DaemonGrpcClient::VersionInfo DaemonGrpcClient::convertVersionInfo(const bunker::daemon::v1::SystemInfoResponse_VersionInfo &pbVersion) {
    VersionInfo version;
    version.daemonVersion = QString::fromStdString(pbVersion.daemon_version());
    version.apiVersion = QString::fromStdString(pbVersion.api_version());
    version.buildTimestamp = QString::fromStdString(pbVersion.build_timestamp());
    version.gitCommit = QString::fromStdString(pbVersion.git_commit());
    
    return version;
}

// ============================================================================
// PHASE 2.2 - MINING CONTROL OPERATIONS
// ============================================================================

void DaemonGrpcClient::startMiningOperation(const QString &algorithm, 
                                           const QString &poolUrl, 
                                           const QString &walletAddress) {
    if (!m_connected || !m_grpcStub) {
        emit miningError("Not connected to daemon - cannot start mining");
        return;
    }
    
    if (m_miningActive.load()) {
        emit miningError("Mining already active - stop current mining before starting new operation");
        return;
    }
    
    qDebug() << "Starting mining operation with algorithm:" << algorithm << "pool:" << poolUrl;
    
    try {
        ClientContext context;
        bunker::daemon::v1::StartMiningRequest request;
        bunker::daemon::v1::CommandResponse response;
        
        // Configure mining request
        auto* config = request.mutable_config();
        config->set_algorithm(algorithm.toStdString());
        config->set_pool_url(poolUrl.toStdString());
        config->set_pool_port(443); // Default SSL port
        config->set_worker_name("bunker-miner-client");
        
        if (!walletAddress.isEmpty()) {
            config->set_wallet_address(walletAddress.toStdString());
        }
        
        config->set_intensity(1.0f); // Maximum intensity
        
        request.set_stop_existing(true); // Stop any existing mining
        request.set_timeout_seconds(30); // 30 second timeout
        
        // Set timeout for the call
        context.set_deadline(std::chrono::system_clock::now() + std::chrono::seconds(30));
        
        Status status = m_grpcStub->StartMining(&context, request, &response);
        
        if (status.ok()) {
            if (response.status() == bunker::daemon::v1::CommandResponse::STATUS_SUCCESS) {
                updateMiningState(true, algorithm);
                m_currentPoolUrl = poolUrl;
                m_currentWalletAddress = walletAddress;
                
                emit miningStarted(algorithm);
                qDebug() << "Mining started successfully with algorithm:" << algorithm;
                
                // Automatically start telemetry streaming when mining starts
                startTelemetryStream();
                
            } else {
                QString errorMsg = QString("Mining start failed: %1")
                                  .arg(QString::fromStdString(response.message()));
                qWarning() << errorMsg;
                emit miningError(errorMsg);
            }
        } else {
            QString errorMsg = QString("StartMining RPC failed: %1 (%2)")
                              .arg(QString::fromStdString(status.error_message()))
                              .arg(status.error_code());
            qWarning() << errorMsg;
            emit miningError(errorMsg);
        }
    } catch (const std::exception &e) {
        QString errorMsg = QString("Mining start exception: %1").arg(e.what());
        qWarning() << errorMsg;
        emit miningError(errorMsg);
    }
}

void DaemonGrpcClient::stopMiningOperation() {
    if (!m_connected || !m_grpcStub) {
        emit miningError("Not connected to daemon - cannot stop mining");
        return;
    }
    
    qDebug() << "Stopping mining operation";
    
    // Stop telemetry stream first
    stopTelemetryStream();
    
    try {
        ClientContext context;
        bunker::daemon::v1::StopMiningRequest request;
        bunker::daemon::v1::CommandResponse response;
        
        request.set_force_stop(false); // Graceful stop
        request.set_timeout_seconds(15); // 15 second timeout
        
        context.set_deadline(std::chrono::system_clock::now() + std::chrono::seconds(20));
        
        Status status = m_grpcStub->StopMining(&context, request, &response);
        
        if (status.ok()) {
            if (response.status() == bunker::daemon::v1::CommandResponse::STATUS_SUCCESS) {
                updateMiningState(false);
                emit miningStopped();
                qDebug() << "Mining stopped successfully";
            } else {
                QString errorMsg = QString("Mining stop failed: %1")
                                  .arg(QString::fromStdString(response.message()));
                qWarning() << errorMsg;
                emit miningError(errorMsg);
            }
        } else {
            QString errorMsg = QString("StopMining RPC failed: %1 (%2)")
                              .arg(QString::fromStdString(status.error_message()))
                              .arg(status.error_code());
            qWarning() << errorMsg;
            emit miningError(errorMsg);
        }
    } catch (const std::exception &e) {
        QString errorMsg = QString("Mining stop exception: %1").arg(e.what());
        qWarning() << errorMsg;
        emit miningError(errorMsg);
    }
}

bool DaemonGrpcClient::isMiningActive() const {
    return m_miningActive.load();
}

QString DaemonGrpcClient::getCurrentMiningAlgorithm() const {
    return m_currentAlgorithm;
}

// ============================================================================
// PHASE 2.2 - TELEMETRY STREAMING MANAGEMENT
// ============================================================================

void DaemonGrpcClient::startTelemetryStream() {
    if (!m_connected) {
        emit telemetryStreamError("Not connected to daemon - cannot start telemetry stream");
        return;
    }
    
    if (m_telemetryStreamActive.load()) {
        qDebug() << "Telemetry stream already active";
        return;
    }
    
    qDebug() << "Starting telemetry stream";
    
    setupTelemetryWorker();
    
    if (m_telemetryWorker && m_telemetryThread) {
        m_telemetryWorker->startTelemetryStream(m_daemonAddress);
        m_telemetryStreamActive.store(true);
    }
}

void DaemonGrpcClient::stopTelemetryStream() {
    if (!m_telemetryStreamActive.load()) {
        return;
    }
    
    qDebug() << "Stopping telemetry stream";
    
    if (m_telemetryWorker) {
        m_telemetryWorker->stopTelemetryStream();
    }
    
    m_telemetryStreamActive.store(false);
}

bool DaemonGrpcClient::isTelemetryStreamActive() const {
    return m_telemetryStreamActive.load();
}

// ============================================================================
// PHASE 2.2 - PRIVATE IMPLEMENTATION METHODS
// ============================================================================

void DaemonGrpcClient::initializeMiningState() {
    m_miningActive.store(false);
    m_currentAlgorithm.clear();
    m_currentPoolUrl.clear();
    m_currentWalletAddress.clear();
    m_telemetryStreamActive.store(false);
}

void DaemonGrpcClient::updateMiningState(bool active, const QString &algorithm) {
    m_miningActive.store(active);
    
    if (active) {
        m_currentAlgorithm = algorithm;
    } else {
        m_currentAlgorithm.clear();
        m_currentPoolUrl.clear();
        m_currentWalletAddress.clear();
    }
}

void DaemonGrpcClient::setupTelemetryWorker() {
    if (m_telemetryWorker && m_telemetryThread) {
        return; // Already set up
    }
    
    // Create worker thread
    m_telemetryThread = new QThread(this);
    
    // Create worker object
    m_telemetryWorker = new TelemetryWorker();
    
    // Move worker to thread (this must be done before connecting signals)
    m_telemetryWorker->moveToThread(m_telemetryThread);
    
    // Connect worker signals to client slots for signal relay
    connect(m_telemetryWorker, &TelemetryWorker::streamStarted,
            this, &DaemonGrpcClient::onTelemetryStreamStarted);
    connect(m_telemetryWorker, &TelemetryWorker::streamStopped,
            this, &DaemonGrpcClient::onTelemetryStreamStopped);
    connect(m_telemetryWorker, &TelemetryWorker::streamError,
            this, &DaemonGrpcClient::onTelemetryStreamError);
    
    // Connect worker telemetry data signal directly to MainWindow
    // (MainWindow will connect to this signal for real-time updates)
    
    // Connect thread lifecycle
    connect(m_telemetryThread, &QThread::started, m_telemetryWorker, &TelemetryWorker::processTelemetryStream);
    connect(m_telemetryThread, &QThread::finished, m_telemetryWorker, &QObject::deleteLater);
    
    // Start the thread
    m_telemetryThread->start();
    
    qDebug() << "Telemetry worker thread initialized and started";
}

void DaemonGrpcClient::cleanupTelemetryWorker() {
    if (m_telemetryThread && m_telemetryThread->isRunning()) {
        qDebug() << "Cleaning up telemetry worker thread";
        
        // Stop the thread gracefully
        m_telemetryThread->quit();
        
        if (!m_telemetryThread->wait(3000)) { // Wait up to 3 seconds
            qWarning() << "Telemetry thread did not shut down gracefully, terminating";
            m_telemetryThread->terminate();
            m_telemetryThread->wait(1000);
        }
    }
    
    m_telemetryWorker = nullptr; // Will be deleted by thread
    
    if (m_telemetryThread) {
        m_telemetryThread->deleteLater();
        m_telemetryThread = nullptr;
    }
    
    m_telemetryStreamActive.store(false);
    
    qDebug() << "Telemetry worker cleanup completed";
}

// ============================================================================
// PHASE 2.2 - TELEMETRY WORKER SIGNAL RELAY SLOTS
// ============================================================================

void DaemonGrpcClient::onTelemetryStreamStarted() {
    qDebug() << "Telemetry stream started successfully";
    emit telemetryStreamStarted();
}

void DaemonGrpcClient::onTelemetryStreamStopped() {
    qDebug() << "Telemetry stream stopped";
    m_telemetryStreamActive.store(false);
    emit telemetryStreamStopped();
}

void DaemonGrpcClient::onTelemetryStreamError(const QString &error) {
    qWarning() << "Telemetry stream error:" << error;
    m_telemetryStreamActive.store(false);
    emit telemetryStreamError(error);
}