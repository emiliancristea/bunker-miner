#pragma once

#include <QWidget>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QGridLayout>
#include <QLabel>
#include <QProgressBar>
#include <QFrame>
#include <QTimer>
#include "TelemetryWorker.h"

/**
 * DeviceTelemetryWidget - Phase 2.2
 * 
 * Custom widget that displays real-time telemetry data for a single mining device.
 * This widget provides a comprehensive view of device performance metrics including
 * hashrate, power consumption, temperature, share statistics, and device status.
 * 
 * Features:
 * - Real-time telemetry display with automatic updates
 * - Visual status indicators (colors, progress bars)
 * - Comprehensive device metrics in organized layout
 * - Professional mining operation aesthetics
 * - Efficient UI updates to prevent performance bottlenecks
 */
class DeviceTelemetryWidget : public QWidget {
    Q_OBJECT

public:
    explicit DeviceTelemetryWidget(const QString &deviceId, const QString &deviceName, QWidget *parent = nullptr);
    ~DeviceTelemetryWidget() override = default;

    // Device identification
    QString getDeviceId() const { return m_deviceId; }
    QString getDeviceName() const { return m_deviceName; }
    
    // Telemetry updates
    void updateTelemetry(const TelemetryWorker::TelemetryData &telemetryData);
    void setOfflineState();
    void clearData();

    // Visual states
    void setMiningState(bool isMining);
    
signals:
    void deviceClicked(const QString &deviceId);

protected:
    void mousePressEvent(QMouseEvent *event) override;
    void paintEvent(QPaintEvent *event) override;

private slots:
    void onUpdateTimer();

private:
    void setupUI();
    void updateStatusIndicator(const QString &status);
    void updateSharesIndicator(float acceptanceRate);
    QString formatHashrate(double hashrateMhs);
    QString formatUptime(qint64 timestamp);
    QColor getStatusColor(const QString &status);
    QColor getTemperatureColor(int temperature);
    
    // Device information
    QString m_deviceId;
    QString m_deviceName;
    
    // UI components - Header
    QVBoxLayout *m_mainLayout;
    QFrame *m_headerFrame;
    QHBoxLayout *m_headerLayout;
    QLabel *m_deviceNameLabel;
    QLabel *m_statusIndicator;
    QLabel *m_algorithmLabel;
    
    // UI components - Metrics grid
    QFrame *m_metricsFrame;
    QGridLayout *m_metricsLayout;
    
    // Primary metrics
    QLabel *m_hashrateLabel;
    QLabel *m_hashrateValue;
    QLabel *m_powerLabel;
    QLabel *m_powerValue;
    QLabel *m_tempLabel;
    QLabel *m_tempValue;
    
    // Secondary metrics
    QLabel *m_fanLabel;
    QLabel *m_fanValue;
    QProgressBar *m_utilizationBar;
    QLabel *m_utilizationLabel;
    QProgressBar *m_memoryBar;
    QLabel *m_memoryLabel;
    
    // Share statistics
    QFrame *m_sharesFrame;
    QHBoxLayout *m_sharesLayout;
    QLabel *m_acceptedSharesLabel;
    QLabel *m_rejectedSharesLabel;
    QLabel *m_acceptanceRateLabel;
    QLabel *m_shareTimeLabel;
    
    // Pool information
    QLabel *m_poolStatusLabel;
    QLabel *m_poolUrlLabel;
    
    // Last telemetry data (for comparison and delta calculations)
    TelemetryWorker::TelemetryData m_lastTelemetryData;
    qint64 m_lastUpdateTime;
    
    // Update management
    QTimer *m_updateTimer;
    bool m_isMining;
    bool m_hasData;
    
    // Visual state
    QString m_currentStatus;
    QColor m_currentStatusColor;
    
    // Constants
    static constexpr int UPDATE_INTERVAL_MS = 1000; // 1 second UI updates
    static constexpr int STALE_DATA_THRESHOLD_MS = 10000; // 10 seconds
};