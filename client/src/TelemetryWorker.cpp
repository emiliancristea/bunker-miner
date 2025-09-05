#include "TelemetryWorker.h"
#include <QDebug>
#include <QDateTime>
#include <QThread>
#include <QCoreApplication>
#include <google/protobuf/empty.pb.h>
#include <chrono>

/**
 * TelemetryWorker implementation - Phase 2.2
 * 
 * Implements real-time telemetry streaming from the BUNKER MINER daemon
 * using gRPC streaming in a dedicated worker thread. This ensures the
 * UI remains responsive while processing high-frequency telemetry data.
 */

TelemetryWorker::TelemetryWorker(QObject *parent)
    : QObject(parent)
    , m_grpcChannel(nullptr)
    , m_grpcStub(nullptr)
    , m_streamContext(nullptr)
    , m_telemetryReader(nullptr)
    , m_streamActive(false)
    , m_shouldStop(false)
    , m_daemonAddress("127.0.0.1:50051")
    , m_healthCheckTimer(new QTimer(this))
    , m_messagesReceived(0)
    , m_lastMessageTime(0)
{
    qDebug() << "TelemetryWorker initialized for real-time daemon streaming";
    
    // Configure health check timer (runs in worker thread when moved)
    m_healthCheckTimer->setSingleShot(false);
    m_healthCheckTimer->setInterval(HEALTH_CHECK_INTERVAL_MS);
    connect(m_healthCheckTimer, &QTimer::timeout, this, &TelemetryWorker::checkStreamHealth);
    
    // Note: moveToThread() will be called by the main thread before starting
}

TelemetryWorker::~TelemetryWorker() {
    stopTelemetryStream();
    cleanupGrpcClient();
}

void TelemetryWorker::startTelemetryStream(const QString &daemonAddress) {
    QMutexLocker locker(&m_mutex);
    
    if (m_streamActive.load()) {
        qDebug() << "Telemetry stream already active, ignoring start request";
        return;
    }
    
    m_daemonAddress = daemonAddress;
    m_shouldStop.store(false);
    
    qDebug() << "Starting telemetry stream for daemon at:" << daemonAddress;
    
    // Initialize will be called in the worker thread
    QMetaObject::invokeMethod(this, "processTelemetryStream", Qt::QueuedConnection);
}

void TelemetryWorker::stopTelemetryStream() {
    qDebug() << "Stopping telemetry stream";
    
    m_shouldStop.store(true);
    m_healthCheckTimer->stop();
    
    QMutexLocker locker(&m_mutex);
    
    // Cancel the stream context to interrupt blocking read
    if (m_streamContext) {
        m_streamContext->TryCancel();
    }
    
    // Reset stream state
    resetStreamState();
    
    m_stopCondition.wakeAll();
    
    if (m_streamActive.exchange(false)) {
        emit streamStopped();
        qDebug() << "Telemetry stream stopped successfully";
    }
}

bool TelemetryWorker::isStreamActive() const {
    return m_streamActive.load();
}

void TelemetryWorker::processTelemetryStream() {
    qDebug() << "TelemetryWorker::processTelemetryStream() starting in thread:" << QThread::currentThreadId();
    
    try {
        initializeGrpcClient();
        
        while (!m_shouldStop.load()) {
            if (establishTelemetryStream()) {
                m_streamActive.store(true);
                emit streamStarted();
                emit connectionEstablished();
                
                // Start health monitoring
                m_healthCheckTimer->start();
                
                qDebug() << "Telemetry stream established, processing messages";
                processTelemetryMessages();
                
                // Stream ended, clean up
                m_healthCheckTimer->stop();
            } else {
                emit connectionLost();
                
                if (!m_shouldStop.load()) {
                    qDebug() << "Failed to establish telemetry stream, retrying in" << RECONNECT_DELAY_MS << "ms";
                    QThread::msleep(RECONNECT_DELAY_MS);
                }
            }
            
            resetStreamState();
        }
    } catch (const std::exception &e) {
        QString errorMsg = QString("Telemetry worker exception: %1").arg(e.what());
        qWarning() << errorMsg;
        emit streamError(errorMsg);
    }
    
    m_streamActive.store(false);
    cleanupGrpcClient();
    
    qDebug() << "TelemetryWorker::processTelemetryStream() completed";
}

void TelemetryWorker::checkStreamHealth() {
    if (!m_streamActive.load()) {
        return;
    }
    
    qint64 currentTime = QDateTime::currentMSecsSinceEpoch();
    qint64 lastMessage = m_lastMessageTime.load();
    
    // Check if we've received messages recently (within 30 seconds)
    if (lastMessage > 0 && (currentTime - lastMessage) > (STREAM_TIMEOUT_SECONDS * 1000)) {
        qWarning() << "Telemetry stream appears stale, last message:" << (currentTime - lastMessage) << "ms ago";
        emit streamError("Telemetry stream timeout - no data received");
        
        // Trigger reconnection
        if (m_streamContext) {
            m_streamContext->TryCancel();
        }
    }
}

void TelemetryWorker::initializeGrpcClient() {
    QMutexLocker locker(&m_mutex);
    
    // Security check: Only allow localhost connections
    if (!m_daemonAddress.startsWith("127.0.0.1:") && !m_daemonAddress.startsWith("localhost:")) {
        QString errorMsg = "Security: Only localhost connections allowed for telemetry streaming";
        qWarning() << errorMsg;
        emit streamError(errorMsg);
        return;
    }
    
    // Create gRPC channel with appropriate settings for streaming
    grpc::ChannelArguments channelArgs;
    channelArgs.SetMaxReceiveMessageSize(16 * 1024 * 1024); // 16MB for large telemetry batches
    channelArgs.SetMaxSendMessageSize(1 * 1024 * 1024);     // 1MB max send
    channelArgs.SetKeepAliveTime(10 * 1000);                // 10 seconds keep-alive
    channelArgs.SetKeepAliveTimeout(3 * 1000);              // 3 seconds timeout
    channelArgs.SetKeepAlivePermitWithoutCalls(true);
    channelArgs.SetInt(GRPC_ARG_HTTP2_MIN_RECV_PING_INTERVAL_WITHOUT_DATA_MS, 5000);
    channelArgs.SetInt(GRPC_ARG_HTTP2_MIN_SENT_PING_INTERVAL_WITHOUT_DATA_MS, 10000);
    
    m_grpcChannel = grpc::CreateCustomChannel(m_daemonAddress.toStdString(), 
                                            grpc::InsecureChannelCredentials(),
                                            channelArgs);
    
    if (!m_grpcChannel) {
        throw std::runtime_error("Failed to create gRPC channel for telemetry");
    }
    
    m_grpcStub = bunker::daemon::v1::BunkerMinerDaemon::NewStub(m_grpcChannel);
    
    if (!m_grpcStub) {
        throw std::runtime_error("Failed to create gRPC stub for telemetry");
    }
    
    qDebug() << "gRPC client initialized for telemetry streaming";
}

void TelemetryWorker::cleanupGrpcClient() {
    QMutexLocker locker(&m_mutex);
    
    resetStreamState();
    m_grpcStub.reset();
    m_grpcChannel.reset();
    
    qDebug() << "gRPC client cleaned up";
}

void TelemetryWorker::resetStreamState() {
    m_telemetryReader.reset();
    m_streamContext.reset();
}

bool TelemetryWorker::establishTelemetryStream() {
    if (!m_grpcStub || m_shouldStop.load()) {
        return false;
    }
    
    try {
        QMutexLocker locker(&m_mutex);
        
        // Create fresh context for this stream
        m_streamContext = std::make_unique<grpc::ClientContext>();
        
        // Set deadline for the stream (no deadline = infinite stream)
        // We'll rely on keep-alive and health checks instead
        
        google::protobuf::Empty request;
        
        // Start the streaming RPC
        m_telemetryReader = m_grpcStub->StreamTelemetry(m_streamContext.get(), request);
        
        if (!m_telemetryReader) {
            qWarning() << "Failed to create telemetry stream reader";
            return false;
        }
        
        qDebug() << "Telemetry stream established successfully";
        return true;
        
    } catch (const std::exception &e) {
        qWarning() << "Exception establishing telemetry stream:" << e.what();
        return false;
    }
}

void TelemetryWorker::processTelemetryMessages() {
    if (!m_telemetryReader) {
        return;
    }
    
    bunker::daemon::v1::Telemetry telemetryMsg;
    
    qDebug() << "Starting telemetry message processing loop";
    
    try {
        while (!m_shouldStop.load() && m_telemetryReader->Read(&telemetryMsg)) {
            // Update statistics
            m_messagesReceived.fetch_add(1);
            m_lastMessageTime.store(QDateTime::currentMSecsSinceEpoch());
            
            // Convert protobuf to Qt structure
            TelemetryData telemetryData = convertTelemetryData(telemetryMsg);
            
            // Emit signal to main thread (thread-safe)
            emit telemetryReceived(telemetryData);
            
            // Allow other threads to run and check for stop condition
            QCoreApplication::processEvents();
            
            if (m_messagesReceived.load() % 100 == 0) {
                qDebug() << "Processed" << m_messagesReceived.load() << "telemetry messages";
            }
        }
        
        qDebug() << "Telemetry message loop ended naturally";
        
        // Check final status
        grpc::Status status = m_telemetryReader->Finish();
        if (!status.ok() && !m_shouldStop.load()) {
            handleStreamError(status);
        }
        
    } catch (const std::exception &e) {
        QString errorMsg = QString("Exception in telemetry message processing: %1").arg(e.what());
        qWarning() << errorMsg;
        emit streamError(errorMsg);
    }
}

void TelemetryWorker::handleStreamError(const grpc::Status &status) {
    QString errorMsg = QString("Telemetry stream error: %1 (code: %2)")
                       .arg(QString::fromStdString(status.error_message()))
                       .arg(status.error_code());
    
    qWarning() << errorMsg;
    emit streamError(errorMsg);
    emit connectionLost();
}

TelemetryWorker::TelemetryData TelemetryWorker::convertTelemetryData(const bunker::daemon::v1::Telemetry &pbTelemetry) {
    TelemetryData data;
    
    // Basic device information
    data.deviceId = QString::fromStdString(pbTelemetry.device_id());
    data.algorithm = QString::fromStdString(pbTelemetry.algorithm());
    data.timestamp = QDateTime::currentMSecsSinceEpoch();
    
    // Performance metrics
    data.hashrateMhs = pbTelemetry.hashrate_mhs();
    data.powerWatts = pbTelemetry.power_watts();
    data.temperatureCelsius = pbTelemetry.temperature_celsius();
    data.fanSpeedPercent = pbTelemetry.fan_speed_percent();
    data.utilizationPercent = pbTelemetry.utilization_percent();
    data.memoryUtilizationPercent = pbTelemetry.memory_utilization_percent();
    data.coreClockMhz = pbTelemetry.core_clock_mhz();
    data.memoryClockMhz = pbTelemetry.memory_clock_mhz();
    
    // Share statistics
    if (pbTelemetry.has_shares()) {
        data.acceptedShares = pbTelemetry.shares().accepted();
        data.rejectedShares = pbTelemetry.shares().rejected();
        data.staleShares = pbTelemetry.shares().stale();
        data.acceptanceRate = pbTelemetry.shares().acceptance_rate();
        data.avgShareTimeSeconds = pbTelemetry.shares().avg_share_time_seconds();
    } else {
        data.acceptedShares = 0;
        data.rejectedShares = 0;
        data.staleShares = 0;
        data.acceptanceRate = 0.0f;
        data.avgShareTimeSeconds = 0.0f;
    }
    
    // Status information
    switch (pbTelemetry.device_status()) {
        case bunker::daemon::v1::Telemetry::DEVICE_STATUS_IDLE:
            data.deviceStatus = "Idle";
            break;
        case bunker::daemon::v1::Telemetry::DEVICE_STATUS_MINING:
            data.deviceStatus = "Mining";
            break;
        case bunker::daemon::v1::Telemetry::DEVICE_STATUS_ERROR:
            data.deviceStatus = "Error";
            break;
        case bunker::daemon::v1::Telemetry::DEVICE_STATUS_THERMAL_THROTTLING:
            data.deviceStatus = "Thermal Throttling";
            break;
        case bunker::daemon::v1::Telemetry::DEVICE_STATUS_POWER_THROTTLING:
            data.deviceStatus = "Power Throttling";
            break;
        case bunker::daemon::v1::Telemetry::DEVICE_STATUS_OFFLINE:
            data.deviceStatus = "Offline";
            break;
        default:
            data.deviceStatus = "Unknown";
            break;
    }
    
    switch (pbTelemetry.pool_status()) {
        case bunker::daemon::v1::Telemetry::POOL_STATUS_CONNECTED:
            data.poolStatus = "Connected";
            break;
        case bunker::daemon::v1::Telemetry::POOL_STATUS_CONNECTING:
            data.poolStatus = "Connecting";
            break;
        case bunker::daemon::v1::Telemetry::POOL_STATUS_DISCONNECTED:
            data.poolStatus = "Disconnected";
            break;
        case bunker::daemon::v1::Telemetry::POOL_STATUS_ERROR:
            data.poolStatus = "Error";
            break;
        default:
            data.poolStatus = "Unknown";
            break;
    }
    
    data.poolUrl = QString::fromStdString(pbTelemetry.pool_url());
    data.errorMessage = QString::fromStdString(pbTelemetry.error_message());
    
    return data;
}