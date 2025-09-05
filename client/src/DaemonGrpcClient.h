#pragma once

#include <QObject>
#include <QString>
#include <QThread>
#include <QTimer>
#include <memory>
#include <vector>
#include <atomic>

// gRPC includes
#include <grpcpp/grpcpp.h>
#include <grpcpp/health_check_service_interface.h>
#include <grpcpp/ext/proto_server_reflection_plugin.h>

// Generated Protocol Buffer includes
#include "daemon_api.v1.pb.h"
#include "daemon_api.v1.grpc.pb.h"

// Forward declaration
class TelemetryWorker;

// Forward declarations
using grpc::Channel;
using grpc::ClientContext;
using grpc::Status;

/**
 * gRPC client for communicating with the BUNKER MINER daemon - Phase 2.2
 * 
 * This class provides a Qt-friendly wrapper around the generated gRPC client stubs,
 * handling connection management, error handling, data conversion, and real-time
 * telemetry streaming with dedicated worker thread management.
 * 
 * Phase 2.2 Enhancements:
 * - Real-time telemetry streaming with TelemetryWorker integration
 * - Complete mining control operations (start/stop with proper state management)
 * - Thread-safe telemetry data relay to UI components
 * - Mining state tracking and validation
 * 
 * Security: Connects to localhost only by default, preventing accidental 
 * connection to malicious remote daemons.
 */
class DaemonGrpcClient : public QObject {
    Q_OBJECT

public:
    // Device information structure (converted from protobuf)
    struct DeviceInfo {
        QString deviceId;
        QString name;
        QString vendor;
        QString deviceType;
        uint32_t vramMb;
        uint32_t coreCount;
        QString driverVersion;
        QString computeCapability;
        uint32_t baseClockMhz;
        uint32_t memoryClockMhz;
        uint32_t powerLimitWatts;
        QStringList capabilities;
    };

    // System information structure (converted from protobuf)  
    struct SystemInfo {
        QString osName;
        QString osVersion;
        uint32_t totalMemoryGb;
        uint32_t availableMemoryGb;
        QString cpuName;
        uint32_t cpuCores;
        uint32_t cpuThreads;
        uint64_t uptimeSeconds;
    };

    // Version information structure
    struct VersionInfo {
        QString daemonVersion;
        QString apiVersion;
        QString buildTimestamp;
        QString gitCommit;
    };
    
    // Profitability information structure (Phase 2.4)
    struct ProfitabilityInfo {
        QString algorithm;
        QString coin;
        double revenueEurPerDay;
        double costEurPerDay;
        double profitEurPerDay;
        double networkDifficulty;
        double coinPriceEur;
        QDateTime calculatedAt;
        float confidence;
        QString dataSource;
    };

    explicit DaemonGrpcClient(QObject *parent = nullptr);
    ~DaemonGrpcClient() override;

    // Connection management
    bool connectToDaemon(const QString &address = "127.0.0.1:50051");
    void disconnectFromDaemon();
    bool isConnected() const;

    // Daemon API calls
    void getSystemInfo();
    void getHealthCheck();
    void startMining(const QString &algorithm = "");
    void stopMining();

    // Configuration management
    void getConfiguration();
    void setConfiguration(const QString &configJson);
    
    // Mining control operations (Phase 2.2)
    void startMiningOperation(const QString &algorithm = "RandomX", 
                             const QString &poolUrl = "pool.supportxmr.com:443",
                             const QString &walletAddress = "");
    void stopMiningOperation();
    bool isMiningActive() const;
    QString getCurrentMiningAlgorithm() const;
    
    // Telemetry streaming management (Phase 2.2)
    void startTelemetryStream();
    void stopTelemetryStream();
    bool isTelemetryStreamActive() const;
    
    // Profitability operations (Phase 2.4)
    void getProfitabilityData();
    void startAutoMining(const QString &walletAddress = "");
    void stopAutoMining();

signals:
    // Connection signals
    void connected();
    void disconnected();
    void connectionError(const QString &error);

    // Data signals
    void systemInfoReceived(const std::vector<DeviceInfo> &devices, 
                          const SystemInfo &systemInfo,
                          const VersionInfo &versionInfo);
    void healthCheckReceived(const QString &status, const QStringList &componentHealth);
    void commandResponse(const QString &response);
    void configurationReceived(const QString &configJson);
    
    // Mining control signals (Phase 2.2)
    void miningStarted(const QString &algorithm);
    void miningStopped();
    void miningError(const QString &error);
    
    // Telemetry signals (Phase 2.2) - relayed from TelemetryWorker
    void telemetryStreamStarted();
    void telemetryStreamStopped();
    void telemetryStreamError(const QString &error);
    
    // Profitability signals (Phase 2.4)
    void profitabilityDataReceived(const QVector<ProfitabilityInfo> &profitabilityData, 
                                  const QString &recommendedAlgorithm);

private slots:
    void checkConnectionHealth();
    
    // Telemetry worker slots (Phase 2.2)
    void onTelemetryStreamStarted();
    void onTelemetryStreamStopped();
    void onTelemetryStreamError(const QString &error);

private:
    void initializeGrpcClient();
    void cleanupGrpcClient();
    bool performHealthCheck();
    
    // Mining state management (Phase 2.2)
    void initializeMiningState();
    void updateMiningState(bool active, const QString &algorithm = "");
    
    // Telemetry worker management (Phase 2.2)
    void setupTelemetryWorker();
    void cleanupTelemetryWorker();
    
    // Convert protobuf messages to Qt structures
    DeviceInfo convertDeviceInfo(const bunker::daemon::v1::DeviceInfo &pbDevice);
    SystemInfo convertSystemInfo(const bunker::daemon::v1::SystemInfoResponse_SystemInfo &pbSystem);
    VersionInfo convertVersionInfo(const bunker::daemon::v1::SystemInfoResponse_VersionInfo &pbVersion);

    // Private members
    bool m_connected;
    QString m_daemonAddress;
    
    // gRPC infrastructure
    std::shared_ptr<grpc::Channel> m_grpcChannel;
    std::unique_ptr<bunker::daemon::v1::BunkerMinerDaemon::Stub> m_grpcStub;
    
    // Health monitoring
    QTimer *m_healthCheckTimer;
    
    // Connection retry logic
    QTimer *m_reconnectTimer;
    int m_connectionRetryCount;
    
    // Mining state tracking (Phase 2.2)
    std::atomic<bool> m_miningActive;
    QString m_currentAlgorithm;
    QString m_currentPoolUrl;
    QString m_currentWalletAddress;
    
    // Telemetry worker management (Phase 2.2)
    TelemetryWorker *m_telemetryWorker;
    QThread *m_telemetryThread;
    std::atomic<bool> m_telemetryStreamActive;
    
    static constexpr int MAX_RETRY_COUNT = 5;
    static constexpr int HEALTH_CHECK_INTERVAL_MS = 10000; // 10 seconds
    static constexpr int RECONNECT_INTERVAL_MS = 5000; // 5 seconds
};