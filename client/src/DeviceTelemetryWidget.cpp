#include "DeviceTelemetryWidget.h"
#include <QMouseEvent>
#include <QPainter>
#include <QDateTime>
#include <QDebug>
#include <QApplication>
#include <QFont>
#include <cmath>

/**
 * DeviceTelemetryWidget implementation - Phase 2.2
 * 
 * Provides a comprehensive real-time display of mining device telemetry
 * with professional aesthetics and efficient UI updates.
 */

DeviceTelemetryWidget::DeviceTelemetryWidget(const QString &deviceId, const QString &deviceName, QWidget *parent)
    : QWidget(parent)
    , m_deviceId(deviceId)
    , m_deviceName(deviceName)
    , m_mainLayout(nullptr)
    , m_headerFrame(nullptr)
    , m_headerLayout(nullptr)
    , m_deviceNameLabel(nullptr)
    , m_statusIndicator(nullptr)
    , m_algorithmLabel(nullptr)
    , m_metricsFrame(nullptr)
    , m_metricsLayout(nullptr)
    , m_hashrateLabel(nullptr)
    , m_hashrateValue(nullptr)
    , m_powerLabel(nullptr)
    , m_powerValue(nullptr)
    , m_tempLabel(nullptr)
    , m_tempValue(nullptr)
    , m_fanLabel(nullptr)
    , m_fanValue(nullptr)
    , m_utilizationBar(nullptr)
    , m_utilizationLabel(nullptr)
    , m_memoryBar(nullptr)
    , m_memoryLabel(nullptr)
    , m_sharesFrame(nullptr)
    , m_sharesLayout(nullptr)
    , m_acceptedSharesLabel(nullptr)
    , m_rejectedSharesLabel(nullptr)
    , m_acceptanceRateLabel(nullptr)
    , m_shareTimeLabel(nullptr)
    , m_poolStatusLabel(nullptr)
    , m_poolUrlLabel(nullptr)
    , m_lastUpdateTime(0)
    , m_updateTimer(new QTimer(this))
    , m_isMining(false)
    , m_hasData(false)
    , m_currentStatus("Idle")
    , m_currentStatusColor(QColor(128, 128, 128))
{
    setupUI();
    
    // Configure update timer
    m_updateTimer->setSingleShot(false);
    m_updateTimer->setInterval(UPDATE_INTERVAL_MS);
    connect(m_updateTimer, &QTimer::timeout, this, &DeviceTelemetryWidget::onUpdateTimer);
    
    // Start the update timer
    m_updateTimer->start();
    
    qDebug() << "DeviceTelemetryWidget created for device:" << deviceId << deviceName;
}

void DeviceTelemetryWidget::setupUI() {
    // Widget properties
    setFixedSize(320, 280);
    setStyleSheet(
        "DeviceTelemetryWidget {"
        "  background-color: #f8f9fa;"
        "  border: 1px solid #dee2e6;"
        "  border-radius: 8px;"
        "}"
        "DeviceTelemetryWidget:hover {"
        "  border: 1px solid #3498db;"
        "  background-color: #f1f8ff;"
        "}"
    );
    
    m_mainLayout = new QVBoxLayout(this);
    m_mainLayout->setSpacing(4);
    m_mainLayout->setContentsMargins(8, 8, 8, 8);
    
    // Header section
    setupHeaderSection();
    
    // Metrics section
    setupMetricsSection();
    
    // Shares statistics section
    setupSharesSection();
    
    // Pool information section
    setupPoolSection();
}

void DeviceTelemetryWidget::setupHeaderSection() {
    m_headerFrame = new QFrame();
    m_headerFrame->setStyleSheet(
        "QFrame {"
        "  background-color: #343a40;"
        "  border-radius: 4px;"
        "  padding: 4px;"
        "}"
        "QLabel {"
        "  color: white;"
        "  font-weight: bold;"
        "}"
    );
    
    m_headerLayout = new QHBoxLayout(m_headerFrame);
    m_headerLayout->setContentsMargins(8, 4, 8, 4);
    
    // Device name
    m_deviceNameLabel = new QLabel(m_deviceName);
    m_deviceNameLabel->setFont(QFont("Arial", 10, QFont::Bold));
    
    // Status indicator (colored dot)
    m_statusIndicator = new QLabel("●");
    m_statusIndicator->setFont(QFont("Arial", 12, QFont::Bold));
    m_statusIndicator->setStyleSheet("color: #6c757d;");
    
    // Algorithm label
    m_algorithmLabel = new QLabel("Idle");
    m_algorithmLabel->setFont(QFont("Arial", 8));
    m_algorithmLabel->setStyleSheet("color: #adb5bd;");
    
    m_headerLayout->addWidget(m_deviceNameLabel);
    m_headerLayout->addWidget(m_statusIndicator);
    m_headerLayout->addStretch();
    m_headerLayout->addWidget(m_algorithmLabel);
    
    m_mainLayout->addWidget(m_headerFrame);

void DeviceTelemetryWidget::setupMetricsSection() {
    m_metricsFrame = new QFrame();
    m_metricsFrame->setStyleSheet(
        "QFrame {"
        "  background-color: white;"
        "  border-radius: 4px;"
        "  padding: 4px;"
        "}"
        "QLabel {"
        "  color: #495057;"
        "  font-size: 11px;"
        "}"
    );
    
    m_metricsLayout = new QGridLayout(m_metricsFrame);
    m_metricsLayout->setSpacing(4);
    m_metricsLayout->setContentsMargins(8, 6, 8, 6);
    
    // Row 0: Hashrate
    m_hashrateLabel = new QLabel("Hashrate:");
    m_hashrateLabel->setFont(QFont("Arial", 9, QFont::Bold));
    m_hashrateValue = new QLabel("0.0 MH/s");
    m_hashrateValue->setFont(QFont("Courier", 10, QFont::Bold));
    m_hashrateValue->setStyleSheet("color: #28a745;");
    
    // Row 1: Power
    m_powerLabel = new QLabel("Power:");
    m_powerValue = new QLabel("0 W");
    m_powerValue->setFont(QFont("Courier", 10));
    
    // Row 2: Temperature
    m_tempLabel = new QLabel("Temp:");
    m_tempValue = new QLabel("0°C");
    m_tempValue->setFont(QFont("Courier", 10));
    
    // Row 3: Fan Speed
    m_fanLabel = new QLabel("Fan:");
    m_fanValue = new QLabel("0%");
    m_fanValue->setFont(QFont("Courier", 10));
    
    // Row 4: GPU Utilization
    m_utilizationLabel = new QLabel("GPU:");
    m_utilizationBar = new QProgressBar();
    m_utilizationBar->setMaximum(100);
    m_utilizationBar->setStyleSheet(
        "QProgressBar {"
        "  border: 1px solid #dee2e6;"
        "  border-radius: 2px;"
        "  text-align: center;"
        "  font-size: 9px;"
        "}"
        "QProgressBar::chunk {"
        "  background-color: #007bff;"
        "  border-radius: 1px;"
        "}"
    );
    
    // Row 5: Memory Utilization
    m_memoryLabel = new QLabel("MEM:");
    m_memoryBar = new QProgressBar();
    m_memoryBar->setMaximum(100);
    m_memoryBar->setStyleSheet(m_utilizationBar->styleSheet());
    
    // Add to grid
    m_metricsLayout->addWidget(m_hashrateLabel, 0, 0);
    m_metricsLayout->addWidget(m_hashrateValue, 0, 1);
    m_metricsLayout->addWidget(m_powerLabel, 0, 2);
    m_metricsLayout->addWidget(m_powerValue, 0, 3);
    
    m_metricsLayout->addWidget(m_tempLabel, 1, 0);
    m_metricsLayout->addWidget(m_tempValue, 1, 1);
    m_metricsLayout->addWidget(m_fanLabel, 1, 2);
    m_metricsLayout->addWidget(m_fanValue, 1, 3);
    
    m_metricsLayout->addWidget(m_utilizationLabel, 2, 0);
    m_metricsLayout->addWidget(m_utilizationBar, 2, 1, 1, 3);
    
    m_metricsLayout->addWidget(m_memoryLabel, 3, 0);
    m_metricsLayout->addWidget(m_memoryBar, 3, 1, 1, 3);
    
    m_mainLayout->addWidget(m_metricsFrame);
}

void DeviceTelemetryWidget::setupSharesSection() {
    m_sharesFrame = new QFrame();
    m_sharesFrame->setStyleSheet(
        "QFrame {"
        "  background-color: #e9ecef;"
        "  border-radius: 4px;"
        "}"
        "QLabel {"
        "  color: #495057;"
        "  font-size: 10px;"
        "  font-weight: bold;"
        "}"
    );
    
    m_sharesLayout = new QHBoxLayout(m_sharesFrame);
    m_sharesLayout->setContentsMargins(8, 4, 8, 4);
    
    m_acceptedSharesLabel = new QLabel("A: 0");
    m_acceptedSharesLabel->setStyleSheet("color: #28a745;");
    
    m_rejectedSharesLabel = new QLabel("R: 0");
    m_rejectedSharesLabel->setStyleSheet("color: #dc3545;");
    
    m_acceptanceRateLabel = new QLabel("Rate: 0.0%");
    m_acceptanceRateLabel->setStyleSheet("color: #17a2b8;");
    
    m_shareTimeLabel = new QLabel("Avg: 0.0s");
    m_shareTimeLabel->setStyleSheet("color: #6c757d;");
    
    m_sharesLayout->addWidget(m_acceptedSharesLabel);
    m_sharesLayout->addWidget(m_rejectedSharesLabel);
    m_sharesLayout->addWidget(m_acceptanceRateLabel);
    m_sharesLayout->addStretch();
    m_sharesLayout->addWidget(m_shareTimeLabel);
    
    m_mainLayout->addWidget(m_sharesFrame);
}

void DeviceTelemetryWidget::setupPoolSection() {
    m_poolStatusLabel = new QLabel("Pool: Disconnected");
    m_poolStatusLabel->setFont(QFont("Arial", 9));
    m_poolStatusLabel->setStyleSheet("color: #6c757d;");
    
    m_poolUrlLabel = new QLabel("No pool configured");
    m_poolUrlLabel->setFont(QFont("Arial", 8));
    m_poolUrlLabel->setStyleSheet("color: #adb5bd;");
    m_poolUrlLabel->setWordWrap(true);
    
    m_mainLayout->addWidget(m_poolStatusLabel);
    m_mainLayout->addWidget(m_poolUrlLabel);
}

void DeviceTelemetryWidget::updateTelemetry(const TelemetryWorker::TelemetryData &telemetryData) {
    if (telemetryData.deviceId != m_deviceId) {
        return; // Not for this device
    }
    
    m_lastTelemetryData = telemetryData;
    m_lastUpdateTime = QDateTime::currentMSecsSinceEpoch();
    m_hasData = true;
    
    // Update hashrate (primary metric)
    m_hashrateValue->setText(formatHashrate(telemetryData.hashrateMhs));
    
    // Update power
    m_powerValue->setText(QString("%1 W").arg(telemetryData.powerWatts));
    
    // Update temperature with color coding
    m_tempValue->setText(QString("%1°C").arg(telemetryData.temperatureCelsius));
    m_tempValue->setStyleSheet(QString("color: %1; font-family: Courier; font-size: 10px;")
                              .arg(getTemperatureColor(telemetryData.temperatureCelsius).name()));
    
    // Update fan speed
    m_fanValue->setText(QString("%1%").arg(telemetryData.fanSpeedPercent));
    
    // Update utilization bars
    m_utilizationBar->setValue(telemetryData.utilizationPercent);
    m_memoryBar->setValue(telemetryData.memoryUtilizationPercent);
    
    // Update shares statistics
    m_acceptedSharesLabel->setText(QString("A: %1").arg(telemetryData.acceptedShares));
    m_rejectedSharesLabel->setText(QString("R: %1").arg(telemetryData.rejectedShares));
    m_acceptanceRateLabel->setText(QString("Rate: %1%").arg(QString::number(telemetryData.acceptanceRate * 100, 'f', 1)));
    m_shareTimeLabel->setText(QString("Avg: %1s").arg(QString::number(telemetryData.avgShareTimeSeconds, 'f', 1)));
    
    // Update status and algorithm
    updateStatusIndicator(telemetryData.deviceStatus);
    m_algorithmLabel->setText(telemetryData.algorithm.isEmpty() ? "Idle" : telemetryData.algorithm);
    
    // Update pool information
    m_poolStatusLabel->setText(QString("Pool: %1").arg(telemetryData.poolStatus));
    m_poolUrlLabel->setText(telemetryData.poolUrl.isEmpty() ? "No pool configured" : telemetryData.poolUrl);
    
    // Update shares indicator color
    updateSharesIndicator(telemetryData.acceptanceRate);
    
    update(); // Trigger repaint for visual updates
}

void DeviceTelemetryWidget::setOfflineState() {
    updateStatusIndicator("Offline");
    m_algorithmLabel->setText("Offline");
    m_hashrateValue->setText("0.0 MH/s");
    m_powerValue->setText("0 W");
    m_tempValue->setText("0°C");
    m_fanValue->setText("0%");
    m_utilizationBar->setValue(0);
    m_memoryBar->setValue(0);
    m_poolStatusLabel->setText("Pool: Disconnected");
    m_poolUrlLabel->setText("Device offline");
    
    m_hasData = false;
    update();
}

void DeviceTelemetryWidget::clearData() {
    setOfflineState();
    
    // Reset shares
    m_acceptedSharesLabel->setText("A: 0");
    m_rejectedSharesLabel->setText("R: 0");
    m_acceptanceRateLabel->setText("Rate: 0.0%");
    m_shareTimeLabel->setText("Avg: 0.0s");
}

void DeviceTelemetryWidget::setMiningState(bool isMining) {
    m_isMining = isMining;
    
    if (!isMining) {
        setOfflineState();
    }
    
    update();
}

void DeviceTelemetryWidget::mousePressEvent(QMouseEvent *event) {
    if (event->button() == Qt::LeftButton) {
        emit deviceClicked(m_deviceId);
    }
    QWidget::mousePressEvent(event);
}

void DeviceTelemetryWidget::paintEvent(QPaintEvent *event) {
    QWidget::paintEvent(event);
    
    // Custom painting for visual effects if needed
    QPainter painter(this);
    
    // Draw mining state indicator
    if (m_isMining && m_hasData) {
        painter.setPen(QPen(QColor(40, 167, 69), 2));
        painter.drawRect(1, 1, width() - 2, height() - 2);
    }
}

void DeviceTelemetryWidget::onUpdateTimer() {
    // Check for stale data
    qint64 currentTime = QDateTime::currentMSecsSinceEpoch();
    
    if (m_hasData && (currentTime - m_lastUpdateTime) > STALE_DATA_THRESHOLD_MS) {
        qDebug() << "Device" << m_deviceId << "data is stale, marking as offline";
        setOfflineState();
    }
    
    // Update UI elements that need periodic refresh
    update();
}

void DeviceTelemetryWidget::updateStatusIndicator(const QString &status) {
    m_currentStatus = status;
    m_currentStatusColor = getStatusColor(status);
    
    m_statusIndicator->setStyleSheet(QString("color: %1; font-size: 12px; font-weight: bold;")
                                    .arg(m_currentStatusColor.name()));
}

void DeviceTelemetryWidget::updateSharesIndicator(float acceptanceRate) {
    QString color;
    
    if (acceptanceRate >= 0.98f) {
        color = "#28a745"; // Green - excellent
    } else if (acceptanceRate >= 0.95f) {
        color = "#ffc107"; // Yellow - good  
    } else if (acceptanceRate >= 0.90f) {
        color = "#fd7e14"; // Orange - warning
    } else {
        color = "#dc3545"; // Red - poor
    }
    
    m_acceptanceRateLabel->setStyleSheet(QString("color: %1; font-weight: bold; font-size: 10px;").arg(color));
}

QString DeviceTelemetryWidget::formatHashrate(double hashrateMhs) {
    if (hashrateMhs >= 1000.0) {
        return QString("%1 GH/s").arg(QString::number(hashrateMhs / 1000.0, 'f', 2));
    } else if (hashrateMhs >= 1.0) {
        return QString("%1 MH/s").arg(QString::number(hashrateMhs, 'f', 2));
    } else if (hashrateMhs >= 0.001) {
        return QString("%1 KH/s").arg(QString::number(hashrateMhs * 1000.0, 'f', 1));
    } else {
        return QString("%1 H/s").arg(QString::number(hashrateMhs * 1000000.0, 'f', 0));
    }
}

QString DeviceTelemetryWidget::formatUptime(qint64 timestamp) {
    qint64 uptime = QDateTime::currentMSecsSinceEpoch() - timestamp;
    qint64 seconds = uptime / 1000;
    
    if (seconds < 60) {
        return QString("%1s").arg(seconds);
    } else if (seconds < 3600) {
        return QString("%1m").arg(seconds / 60);
    } else {
        return QString("%1h %2m").arg(seconds / 3600).arg((seconds % 3600) / 60);
    }
}

QColor DeviceTelemetryWidget::getStatusColor(const QString &status) {
    if (status == "Mining") {
        return QColor(40, 167, 69); // Green
    } else if (status == "Idle") {
        return QColor(108, 117, 125); // Gray
    } else if (status == "Error") {
        return QColor(220, 53, 69); // Red
    } else if (status == "Thermal Throttling") {
        return QColor(255, 193, 7); // Yellow
    } else if (status == "Power Throttling") {
        return QColor(253, 126, 20); // Orange
    } else if (status == "Offline") {
        return QColor(173, 181, 189); // Light gray
    } else {
        return QColor(108, 117, 125); // Default gray
    }
}

QColor DeviceTelemetryWidget::getTemperatureColor(int temperature) {
    if (temperature >= 85) {
        return QColor(220, 53, 69); // Red - danger
    } else if (temperature >= 75) {
        return QColor(253, 126, 20); // Orange - warning
    } else if (temperature >= 65) {
        return QColor(255, 193, 7); // Yellow - caution
    } else {
        return QColor(40, 167, 69); // Green - good
    }
}