#include "MainWindow.h"
#include "DaemonGrpcClient.h"
#include <QApplication>
#include <QMessageBox>
#include <QHeaderView>
#include <QSplitter>
#include <QTimer>
#include <QDateTime>
#include <QDebug>

/**
 * MainWindow implementation for BUNKER MINER Client - Phase 2.1
 * 
 * Features:
 * - Complete UI shell with navigation sidebar
 * - Daemon connection management with error handling
 * - System information display from gRPC API
 * - Proper error states for daemon disconnection
 */

MainWindow::MainWindow(QWidget *parent)
    : QMainWindow(parent)
    , m_centralWidget(nullptr)
    , m_mainLayout(nullptr)
    , m_sidebarWidget(nullptr)
    , m_sidebarLayout(nullptr)
    , m_navigationList(nullptr)
    , m_connectionStatusLabel(nullptr)
    , m_refreshButton(nullptr)
    , m_contentStack(nullptr)
    , m_dashboardPage(nullptr)
    , m_devicesPage(nullptr)
    , m_benchmarksPage(nullptr)
    , m_settingsPage(nullptr)
    , m_dashboardLayout(nullptr)
    , m_daemonStatusLabel(nullptr)
    , m_systemInfoTree(nullptr)
    , m_logOutput(nullptr)
    , m_devicesLayout(nullptr)
    , m_devicesTree(nullptr)
    , m_statusBarLabel(nullptr)
    , m_isConnectedToDaemon(false)
{
    setupUI();
    initializeDaemonClient();
    
    // Auto-connect to daemon on startup
    QTimer::singleShot(1000, [this]() {
        m_daemonClient->connectToDaemon();
    });
}

void MainWindow::setupUI() {
    setWindowTitle("BUNKER MINER Client v2.1.0");
    setMinimumSize(1000, 700);
    resize(1200, 800);
    
    m_centralWidget = new QWidget(this);
    setCentralWidget(m_centralWidget);
    
    m_mainLayout = new QHBoxLayout(m_centralWidget);
    m_mainLayout->setSpacing(0);
    m_mainLayout->setContentsMargins(0, 0, 0, 0);
    
    setupNavigationSidebar();
    setupMainContent();
    setupStatusBar();
    
    // Set initial connection status
    updateConnectionStatus("Connecting to daemon...", false);
}

void MainWindow::setupNavigationSidebar() {
    m_sidebarWidget = new QWidget();
    m_sidebarWidget->setFixedWidth(200);
    m_sidebarWidget->setStyleSheet(
        "QWidget {"
        "  background-color: #2c3e50;"
        "  color: white;"
        "}"
        "QListWidget {"
        "  border: none;"
        "  background-color: #2c3e50;"
        "  color: white;"
        "  font-size: 14px;"
        "  selection-background-color: #3498db;"
        "}"
        "QListWidget::item {"
        "  padding: 12px 16px;"
        "  border-bottom: 1px solid #34495e;"
        "}"
        "QListWidget::item:hover {"
        "  background-color: #34495e;"
        "}"
        "QListWidget::item:selected {"
        "  background-color: #3498db;"
        "}"
        "QLabel {"
        "  color: white;"
        "  font-size: 12px;"
        "  padding: 8px;"
        "}"
        "QPushButton {"
        "  background-color: #3498db;"
        "  color: white;"
        "  border: none;"
        "  padding: 8px 12px;"
        "  font-size: 12px;"
        "  border-radius: 4px;"
        "  margin: 4px;"
        "}"
        "QPushButton:hover {"
        "  background-color: #2980b9;"
        "}"
    );
    
    m_sidebarLayout = new QVBoxLayout(m_sidebarWidget);
    
    // Application title
    QLabel *titleLabel = new QLabel("BUNKER MINER");
    titleLabel->setAlignment(Qt::AlignCenter);
    titleLabel->setStyleSheet("font-size: 16px; font-weight: bold; margin: 16px 8px; color: #ecf0f1;");
    m_sidebarLayout->addWidget(titleLabel);
    
    // Navigation list
    m_navigationList = new QListWidget();
    m_navigationList->addItem("📊 Dashboard");
    m_navigationList->addItem("🖥️ Devices");
    m_navigationList->addItem("⚡ Benchmarks");
    m_navigationList->addItem("⚙️ Settings");
    m_navigationList->setCurrentRow(0); // Default to Dashboard
    
    connect(m_navigationList, &QListWidget::currentRowChanged, 
            this, &MainWindow::onNavigationChanged);
    
    m_sidebarLayout->addWidget(m_navigationList);
    
    // Connection status
    m_connectionStatusLabel = new QLabel("Status: Disconnected");
    m_connectionStatusLabel->setWordWrap(true);
    m_connectionStatusLabel->setStyleSheet("background-color: #e74c3c; padding: 8px; border-radius: 4px; margin: 4px;");
    m_sidebarLayout->addWidget(m_connectionStatusLabel);
    
    // Refresh button
    m_refreshButton = new QPushButton("Refresh Connection");
    connect(m_refreshButton, &QPushButton::clicked, this, &MainWindow::onRefreshSystemInfo);
    m_sidebarLayout->addWidget(m_refreshButton);
    
    m_sidebarLayout->addStretch();
    
    m_mainLayout->addWidget(m_sidebarWidget);
}

void MainWindow::setupMainContent() {
    m_contentStack = new QStackedWidget();
    m_contentStack->setStyleSheet(
        "QWidget {"
        "  background-color: white;"
        "}"
        "QLabel {"
        "  color: #2c3e50;"
        "}"
    );
    
    // Dashboard page
    m_dashboardPage = new QWidget();
    m_dashboardLayout = new QVBoxLayout(m_dashboardPage);
    
    QLabel *dashboardTitle = new QLabel("Dashboard");
    dashboardTitle->setStyleSheet("font-size: 24px; font-weight: bold; margin: 16px; color: #2c3e50;");
    m_dashboardLayout->addWidget(dashboardTitle);
    
    m_daemonStatusLabel = new QLabel("Daemon Status: Connecting...");
    m_daemonStatusLabel->setStyleSheet("font-size: 16px; margin: 8px; padding: 12px; background-color: #f8f9fa; border-left: 4px solid #ffc107;");
    m_dashboardLayout->addWidget(m_daemonStatusLabel);
    
    // System information tree
    QLabel *sysInfoLabel = new QLabel("System Information:");
    sysInfoLabel->setStyleSheet("font-size: 16px; font-weight: bold; margin: 16px 8px 8px 8px;");
    m_dashboardLayout->addWidget(sysInfoLabel);
    
    m_systemInfoTree = new QTreeWidget();
    m_systemInfoTree->setHeaderLabels({"Property", "Value"});
    m_systemInfoTree->header()->setStretchLastSection(true);
    m_systemInfoTree->setAlternatingRowColors(true);
    m_dashboardLayout->addWidget(m_systemInfoTree);
    
    // Log output
    QLabel *logLabel = new QLabel("Connection Log:");
    logLabel->setStyleSheet("font-size: 16px; font-weight: bold; margin: 16px 8px 8px 8px;");
    m_dashboardLayout->addWidget(logLabel);
    
    m_logOutput = new QTextEdit();
    m_logOutput->setMaximumHeight(150);
    m_logOutput->setReadOnly(true);
    m_logOutput->setStyleSheet("background-color: #f8f9fa; border: 1px solid #dee2e6; font-family: 'Consolas', monospace; font-size: 11px;");
    m_dashboardLayout->addWidget(m_logOutput);
    
    m_contentStack->addWidget(m_dashboardPage);
    
    // Devices page
    m_devicesPage = new QWidget();
    m_devicesLayout = new QVBoxLayout(m_devicesPage);
    
    QLabel *devicesTitle = new QLabel("Mining Devices");
    devicesTitle->setStyleSheet("font-size: 24px; font-weight: bold; margin: 16px; color: #2c3e50;");
    m_devicesLayout->addWidget(devicesTitle);
    
    m_devicesTree = new QTreeWidget();
    m_devicesTree->setHeaderLabels({"Device", "Type", "Memory", "Status"});
    m_devicesTree->header()->setStretchLastSection(true);
    m_devicesTree->setAlternatingRowColors(true);
    m_devicesLayout->addWidget(m_devicesTree);
    
    m_contentStack->addWidget(m_devicesPage);
    
    // Benchmarks page (placeholder)
    m_benchmarksPage = new QWidget();
    QVBoxLayout *benchmarksLayout = new QVBoxLayout(m_benchmarksPage);
    QLabel *benchmarksTitle = new QLabel("Benchmarks");
    benchmarksTitle->setStyleSheet("font-size: 24px; font-weight: bold; margin: 16px; color: #2c3e50;");
    benchmarksLayout->addWidget(benchmarksTitle);
    QLabel *benchmarksPlaceholder = new QLabel("Benchmarking functionality will be implemented in Phase 2.2");
    benchmarksPlaceholder->setStyleSheet("color: #6c757d; margin: 16px;");
    benchmarksLayout->addWidget(benchmarksPlaceholder);
    benchmarksLayout->addStretch();
    m_contentStack->addWidget(m_benchmarksPage);
    
    // Settings page (placeholder)
    m_settingsPage = new QWidget();
    QVBoxLayout *settingsLayout = new QVBoxLayout(m_settingsPage);
    QLabel *settingsTitle = new QLabel("Settings");
    settingsTitle->setStyleSheet("font-size: 24px; font-weight: bold; margin: 16px; color: #2c3e50;");
    settingsLayout->addWidget(settingsTitle);
    QLabel *settingsPlaceholder = new QLabel("Settings interface will be implemented in Phase 2.3");
    settingsPlaceholder->setStyleSheet("color: #6c757d; margin: 16px;");
    settingsLayout->addWidget(settingsPlaceholder);
    settingsLayout->addStretch();
    m_contentStack->addWidget(m_settingsPage);
    
    m_mainLayout->addWidget(m_contentStack, 1);
}

void MainWindow::setupStatusBar() {
    m_statusBarLabel = new QLabel("Ready");
    statusBar()->addWidget(m_statusBarLabel);
    statusBar()->showMessage("BUNKER MINER Client v2.1.0 - Initializing...");
}

void MainWindow::initializeDaemonClient() {
    m_daemonClient = std::make_unique<DaemonGrpcClient>(this);
    
    // Connect daemon client signals
    connect(m_daemonClient.get(), &DaemonGrpcClient::connected,
            this, &MainWindow::onDaemonConnected);
    connect(m_daemonClient.get(), &DaemonGrpcClient::disconnected,
            this, &MainWindow::onDaemonDisconnected);
    connect(m_daemonClient.get(), &DaemonGrpcClient::connectionError,
            this, &MainWindow::onDaemonConnectionError);
    connect(m_daemonClient.get(), 
            QOverload<const std::vector<DaemonGrpcClient::DeviceInfo>&, 
                     const DaemonGrpcClient::SystemInfo&,
                     const DaemonGrpcClient::VersionInfo&>::of(&DaemonGrpcClient::systemInfoReceived),
            this, &MainWindow::onSystemInfoReceived);
}

void MainWindow::onNavigationChanged(int index) {
    m_contentStack->setCurrentIndex(index);
    
    QString pageName;
    switch (index) {
        case PAGE_DASHBOARD: pageName = "Dashboard"; break;
        case PAGE_DEVICES: pageName = "Devices"; break;
        case PAGE_BENCHMARKS: pageName = "Benchmarks"; break;
        case PAGE_SETTINGS: pageName = "Settings"; break;
        default: pageName = "Unknown"; break;
    }
    
    statusBar()->showMessage(QString("Viewing: %1").arg(pageName));
}

void MainWindow::onDaemonConnected() {
    m_isConnectedToDaemon = true;
    updateConnectionStatus("Connected to daemon", true);
    showConnectedState();
    
    QString logMessage = QString("[%1] Successfully connected to daemon")
                        .arg(QDateTime::currentDateTime().toString("hh:mm:ss"));
    m_logOutput->append(logMessage);
    
    statusBar()->showMessage("Connected to BUNKER MINER daemon");
    
    // Request system information
    m_daemonClient->getSystemInfo();
}

void MainWindow::onDaemonDisconnected() {
    m_isConnectedToDaemon = false;
    updateConnectionStatus("Disconnected from daemon", false);
    showErrorState("Daemon connection lost");
    
    QString logMessage = QString("[%1] Disconnected from daemon")
                        .arg(QDateTime::currentDateTime().toString("hh:mm:ss"));
    m_logOutput->append(logMessage);
    
    statusBar()->showMessage("Disconnected from daemon");
}

void MainWindow::onDaemonConnectionError(const QString &error) {
    m_isConnectedToDaemon = false;
    updateConnectionStatus("Connection failed", false);
    showErrorState(QString("Connection Error: %1").arg(error));
    
    QString logMessage = QString("[%1] Connection error: %2")
                        .arg(QDateTime::currentDateTime().toString("hh:mm:ss"))
                        .arg(error);
    m_logOutput->append(logMessage);
    
    statusBar()->showMessage("Daemon connection failed");
}

void MainWindow::onSystemInfoReceived(const std::vector<DaemonGrpcClient::DeviceInfo> &devices, 
                                      const DaemonGrpcClient::SystemInfo &systemInfo,
                                      const DaemonGrpcClient::VersionInfo &versionInfo) {
    populateSystemInfo(devices, systemInfo, versionInfo);
    
    QString logMessage = QString("[%1] System information received - %2 devices detected")
                        .arg(QDateTime::currentDateTime().toString("hh:mm:ss"))
                        .arg(devices.size());
    m_logOutput->append(logMessage);
}

void MainWindow::onRefreshSystemInfo() {
    if (m_isConnectedToDaemon) {
        m_daemonClient->getSystemInfo();
        statusBar()->showMessage("Refreshing system information...", 2000);
    } else {
        m_daemonClient->connectToDaemon();
        statusBar()->showMessage("Attempting to reconnect to daemon...", 3000);
    }
}

void MainWindow::updateConnectionStatus(const QString &status, bool connected) {
    m_connectionStatusLabel->setText(QString("Status: %1").arg(status));
    
    if (connected) {
        m_connectionStatusLabel->setStyleSheet("background-color: #27ae60; color: white; padding: 8px; border-radius: 4px; margin: 4px;");
        m_refreshButton->setText("Refresh Data");
    } else {
        m_connectionStatusLabel->setStyleSheet("background-color: #e74c3c; color: white; padding: 8px; border-radius: 4px; margin: 4px;");
        m_refreshButton->setText("Reconnect");
    }
}

void MainWindow::populateSystemInfo(const std::vector<DaemonGrpcClient::DeviceInfo> &devices, 
                                     const DaemonGrpcClient::SystemInfo &systemInfo,
                                     const DaemonGrpcClient::VersionInfo &versionInfo) {
    // Clear existing data
    m_systemInfoTree->clear();
    m_devicesTree->clear();
    
    if (devices.empty() && systemInfo.osName.isEmpty()) {
        // Show placeholder data when no real data is available
        QTreeWidgetItem *placeholderItem = new QTreeWidgetItem(m_systemInfoTree);
        placeholderItem->setText(0, "No Data Available");
        placeholderItem->setText(1, "Waiting for daemon connection...");
        placeholderItem->setFont(0, QFont("Arial", 10, QFont::Bold));
        
        return;
    }
    
    // System Information (real data from daemon)
    QTreeWidgetItem *systemItem = new QTreeWidgetItem(m_systemInfoTree);
    systemItem->setText(0, "System Information");
    systemItem->setFont(0, QFont("Arial", 10, QFont::Bold));
    
    if (!systemInfo.osName.isEmpty()) {
        QTreeWidgetItem *osItem = new QTreeWidgetItem(systemItem);
        osItem->setText(0, "Operating System");
        osItem->setText(1, QString("%1 %2").arg(systemInfo.osName).arg(systemInfo.osVersion));
        
        QTreeWidgetItem *memoryItem = new QTreeWidgetItem(systemItem);
        memoryItem->setText(0, "Total Memory");
        memoryItem->setText(1, QString("%1 GB (%2 GB available)")
                              .arg(systemInfo.totalMemoryGb)
                              .arg(systemInfo.availableMemoryGb));
        
        QTreeWidgetItem *cpuItem = new QTreeWidgetItem(systemItem);
        cpuItem->setText(0, "CPU");
        cpuItem->setText(1, QString("%1 (%2 cores, %3 threads)")
                           .arg(systemInfo.cpuName)
                           .arg(systemInfo.cpuCores)
                           .arg(systemInfo.cpuThreads));
        
        QTreeWidgetItem *uptimeItem = new QTreeWidgetItem(systemItem);
        uptimeItem->setText(0, "System Uptime");
        uptimeItem->setText(1, QString("%1 seconds").arg(systemInfo.uptimeSeconds));
    }
    
    // Daemon Information (real data from daemon)
    QTreeWidgetItem *daemonItem = new QTreeWidgetItem(m_systemInfoTree);
    daemonItem->setText(0, "Daemon Information");
    daemonItem->setFont(0, QFont("Arial", 10, QFont::Bold));
    
    if (!versionInfo.daemonVersion.isEmpty()) {
        QTreeWidgetItem *versionItem = new QTreeWidgetItem(daemonItem);
        versionItem->setText(0, "Daemon Version");
        versionItem->setText(1, versionInfo.daemonVersion);
        
        QTreeWidgetItem *apiVersionItem = new QTreeWidgetItem(daemonItem);
        apiVersionItem->setText(0, "API Version");
        apiVersionItem->setText(1, versionInfo.apiVersion);
        
        QTreeWidgetItem *buildItem = new QTreeWidgetItem(daemonItem);
        buildItem->setText(0, "Build Timestamp");
        buildItem->setText(1, versionInfo.buildTimestamp);
        
        if (!versionInfo.gitCommit.isEmpty()) {
            QTreeWidgetItem *commitItem = new QTreeWidgetItem(daemonItem);
            commitItem->setText(0, "Git Commit");
            commitItem->setText(1, versionInfo.gitCommit.left(8)); // Show short commit hash
        }
    }
    
    m_systemInfoTree->expandAll();
    
    // Mining Devices Information (real data from daemon)
    for (const auto &device : devices) {
        QTreeWidgetItem *deviceItem = new QTreeWidgetItem(m_devicesTree);
        deviceItem->setText(0, device.name);
        deviceItem->setText(1, QString("%1 %2").arg(device.vendor).arg(device.deviceType));
        
        if (device.deviceType == "GPU" && device.vramMb > 0) {
            deviceItem->setText(2, QString("%1 MB").arg(device.vramMb));
        } else if (device.deviceType == "CPU") {
            deviceItem->setText(2, "System RAM");
        } else {
            deviceItem->setText(2, "N/A");
        }
        
        deviceItem->setText(3, "Ready");
        
        // Add detailed device information as child items
        if (device.coreCount > 0) {
            QTreeWidgetItem *coresItem = new QTreeWidgetItem(deviceItem);
            coresItem->setText(0, "  Cores");
            coresItem->setText(1, QString::number(device.coreCount));
        }
        
        if (!device.driverVersion.isEmpty()) {
            QTreeWidgetItem *driverItem = new QTreeWidgetItem(deviceItem);
            driverItem->setText(0, "  Driver Version");
            driverItem->setText(1, device.driverVersion);
        }
        
        if (!device.computeCapability.isEmpty()) {
            QTreeWidgetItem *computeItem = new QTreeWidgetItem(deviceItem);
            computeItem->setText(0, "  Compute Capability");
            computeItem->setText(1, device.computeCapability);
        }
        
        if (device.baseClockMhz > 0) {
            QTreeWidgetItem *clockItem = new QTreeWidgetItem(deviceItem);
            clockItem->setText(0, "  Base Clock");
            clockItem->setText(1, QString("%1 MHz").arg(device.baseClockMhz));
        }
        
        if (device.powerLimitWatts > 0) {
            QTreeWidgetItem *powerItem = new QTreeWidgetItem(deviceItem);
            powerItem->setText(0, "  Power Limit");
            powerItem->setText(1, QString("%1 W").arg(device.powerLimitWatts));
        }
        
        if (!device.capabilities.isEmpty()) {
            QTreeWidgetItem *capabilitiesItem = new QTreeWidgetItem(deviceItem);
            capabilitiesItem->setText(0, "  Capabilities");
            capabilitiesItem->setText(1, device.capabilities.join(", "));
        }
    }
    
    if (!devices.empty()) {
        m_devicesTree->expandAll();
    }
}

void MainWindow::showErrorState(const QString &message) {
    m_daemonStatusLabel->setText(QString("Daemon Status: %1").arg(message));
    m_daemonStatusLabel->setStyleSheet("font-size: 16px; margin: 8px; padding: 12px; background-color: #f8d7da; border-left: 4px solid #dc3545; color: #721c24;");
    
    // Clear system info when disconnected
    m_systemInfoTree->clear();
    m_devicesTree->clear();
    
    QTreeWidgetItem *errorItem = new QTreeWidgetItem(m_systemInfoTree);
    errorItem->setText(0, "Connection Error");
    errorItem->setText(1, "Daemon connection failed. Is BUNKER MINER daemon running?");
    errorItem->setFont(0, QFont("Arial", 10, QFont::Bold));
    
    QTreeWidgetItem *helpItem = new QTreeWidgetItem(m_systemInfoTree);
    helpItem->setText(0, "Troubleshooting");
    helpItem->setText(1, "1. Ensure daemon is running on localhost:50051");
    
    QTreeWidgetItem *helpItem2 = new QTreeWidgetItem(m_systemInfoTree);
    helpItem2->setText(0, "");
    helpItem2->setText(1, "2. Click 'Reconnect' to retry connection");
    
    m_systemInfoTree->expandAll();
}

void MainWindow::showConnectedState() {
    m_daemonStatusLabel->setText("Daemon Status: Connected and Ready");
    m_daemonStatusLabel->setStyleSheet("font-size: 16px; margin: 8px; padding: 12px; background-color: #d1edff; border-left: 4px solid #0ea5e9; color: #0c4a6e;");
}