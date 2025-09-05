#pragma once

#include <QObject>
#include <QString>
#include <QThread>
#include <QTimer>
#include <memory>
#include <vector>

// gRPC includes
#include <grpcpp/grpcpp.h>
#include <grpcpp/health_check_service_interface.h>
#include <grpcpp/ext/proto_server_reflection_plugin.h>

// Generated Protocol Buffer includes
#include "daemon_api.v1.pb.h"
#include "daemon_api.v1.grpc.pb.h"

// Forward declarations
using grpc::Channel;
using grpc::ClientContext;
using grpc::Status;

/**
 * gRPC client for communicating with the BUNKER MINER daemon - Phase 2.1
 * 
 * This class provides a Qt-friendly wrapper around the generated gRPC client stubs,
 * handling connection management, error handling, and data conversion between
 * Protocol Buffer messages and Qt types.
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

private slots:
    void checkConnectionHealth();

private:
    void initializeGrpcClient();
    void cleanupGrpcClient();
    bool performHealthCheck();
    
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
    static constexpr int MAX_RETRY_COUNT = 5;
    static constexpr int HEALTH_CHECK_INTERVAL_MS = 10000; // 10 seconds
    static constexpr int RECONNECT_INTERVAL_MS = 5000; // 5 seconds
};