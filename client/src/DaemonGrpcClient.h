#pragma once

#include <QObject>
#include <QString>
#include <QThread>
#include <memory>

/**
 * gRPC client for communicating with the BUNKER MINER daemon
 * 
 * Phase 0.1: Stub implementation
 * Phase 2.1: Full gRPC implementation using generated Protocol Buffer stubs
 */
class DaemonGrpcClient : public QObject {
    Q_OBJECT

public:
    explicit DaemonGrpcClient(QObject *parent = nullptr);
    ~DaemonGrpcClient() override = default;

    // Connection management
    bool connectToDaemon(const QString &address = "127.0.0.1:50051");
    void disconnectFromDaemon();
    bool isConnected() const;

    // Daemon commands (Phase 2.1 implementation)
    void getSystemInfo();
    void startMining(const QString &algorithm = "");
    void stopMining();
    void streamTelemetry();

signals:
    // Connection signals
    void connected();
    void disconnected();
    void connectionError(const QString &error);

    // Data signals (Phase 2.1+)
    void systemInfoReceived(/* SystemInfo data */);
    void telemetryReceived(/* Telemetry data */);
    void commandResponse(const QString &response);

private slots:
    void onConnectionStateChanged();

private:
    void initializeGrpcClient();
    void cleanupGrpcClient();

    // Private members (Phase 2.1 implementation)
    bool m_connected;
    QString m_daemonAddress;
    
    // gRPC client stub will be added in Phase 2.1
    // std::unique_ptr<BunkerMinerDaemon::Stub> m_grpcStub;
};