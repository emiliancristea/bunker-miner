#pragma once

#include <QObject>
#include <QThread>
#include <QTimer>
#include <QMutex>
#include <QWaitCondition>
#include <memory>
#include <atomic>

// gRPC includes
#include <grpcpp/grpcpp.h>
#include <grpcpp/health_check_service_interface.h>

// Generated Protocol Buffer includes
#include "daemon_api.v1.pb.h"
#include "daemon_api.v1.grpc.pb.h"

// Forward declarations
using grpc::Channel;
using grpc::ClientContext;
using grpc::Status;
using grpc::ClientReader;

/**
 * TelemetryWorker - Phase 2.2
 * 
 * Dedicated worker class for handling real-time telemetry streaming from the daemon
 * in a separate thread. This prevents the gRPC streaming operations from blocking
 * the main UI thread, ensuring responsive user interaction.
 * 
 * Key Features:
 * - Runs in separate QThread to avoid UI blocking
 * - Handles gRPC StreamTelemetry RPC calls
 * - Thread-safe signal emission for UI updates
 * - Graceful shutdown and resource cleanup
 * - Automatic reconnection on stream failure
 */
class TelemetryWorker : public QObject {
    Q_OBJECT

public:
    // Telemetry data structure for Qt signal emission
    struct TelemetryData {
        QString deviceId;
        QString algorithm;
        double hashrateMhs;
        uint32_t powerWatts;
        uint32_t temperatureCelsius;
        uint32_t fanSpeedPercent;
        uint32_t utilizationPercent;
        uint32_t memoryUtilizationPercent;
        uint32_t coreClockMhz;
        uint32_t memoryClockMhz;
        
        // Share statistics
        uint64_t acceptedShares;
        uint64_t rejectedShares;
        uint64_t staleShares;
        float acceptanceRate;
        float avgShareTimeSeconds;
        
        // Status information
        QString deviceStatus;
        QString poolStatus;
        QString poolUrl;
        QString errorMessage;
        
        // Timestamp
        qint64 timestamp;
    };

    explicit TelemetryWorker(QObject *parent = nullptr);
    ~TelemetryWorker() override;

    // Control methods (thread-safe)
    void startTelemetryStream(const QString &daemonAddress = "127.0.0.1:50051");
    void stopTelemetryStream();
    bool isStreamActive() const;

signals:
    // Emitted when new telemetry data is received (thread-safe)
    void telemetryReceived(const TelemetryWorker::TelemetryData &telemetryData);
    
    // Stream status signals
    void streamStarted();
    void streamStopped();
    void streamError(const QString &error);
    
    // Connection status signals
    void connectionEstablished();
    void connectionLost();

public slots:
    // Main telemetry streaming loop (runs in worker thread)
    void processTelemetryStream();

private slots:
    // Internal connection monitoring
    void checkStreamHealth();

private:
    void initializeGrpcClient();
    void cleanupGrpcClient();
    void resetStreamState();
    
    // Convert protobuf telemetry to Qt structure
    TelemetryData convertTelemetryData(const bunker::daemon::v1::Telemetry &pbTelemetry);
    
    // Stream management
    bool establishTelemetryStream();
    void processTelemetryMessages();
    void handleStreamError(const grpc::Status &status);
    
    // Thread synchronization
    mutable QMutex m_mutex;
    QWaitCondition m_stopCondition;
    
    // gRPC infrastructure
    std::shared_ptr<grpc::Channel> m_grpcChannel;
    std::unique_ptr<bunker::daemon::v1::BunkerMinerDaemon::Stub> m_grpcStub;
    std::unique_ptr<grpc::ClientContext> m_streamContext;
    std::unique_ptr<grpc::ClientReader<bunker::daemon::v1::Telemetry>> m_telemetryReader;
    
    // Worker state
    std::atomic<bool> m_streamActive;
    std::atomic<bool> m_shouldStop;
    QString m_daemonAddress;
    
    // Health monitoring
    QTimer *m_healthCheckTimer;
    
    // Stream statistics
    std::atomic<uint64_t> m_messagesReceived;
    std::atomic<uint64_t> m_lastMessageTime;
    
    // Constants
    static constexpr int HEALTH_CHECK_INTERVAL_MS = 5000; // 5 seconds
    static constexpr int STREAM_TIMEOUT_SECONDS = 30;
    static constexpr int RECONNECT_DELAY_MS = 2000; // 2 seconds
};