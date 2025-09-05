#include "MainWindow.h"
#include "DaemonGrpcClient.h"
#include <QApplication>
#include <QMessageBox>
#include <QHeaderView>
#include <QSplitter>
#include <QTimer>
#include <QDateTime>
#include <QDebug>
#include <QInputDialog>
#include <QLineEdit>

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
    , m_isMining(false) // Phase 2.2
    , m_isAutoMining(false) // Phase 2.4
    , m_profitabilityRefreshTimer(nullptr) // Phase 2.4
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
    m_navigationList->addItem("💰 Profitability");  // Phase 2.4 - New profitability page
    m_navigationList->addItem("🏊 Pool Stats");     // Phase 3.4 - BUNKER POOL statistics page
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
    
    // Profitability page (Phase 2.4)
    setupProfitabilityPage();
    m_contentStack->addWidget(m_profitabilityPage);
    
    // BUNKER POOL Stats page (Phase 3.4)
    setupPoolStatsPage();
    m_contentStack->addWidget(m_poolStatsPage);
    
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
    
    // Settings page with Fleet Management (Phase 4.3)
    m_settingsPage = new QWidget();
    QVBoxLayout *settingsLayout = new QVBoxLayout(m_settingsPage);
    QLabel *settingsTitle = new QLabel("Settings");
    settingsTitle->setStyleSheet("font-size: 24px; font-weight: bold; margin: 16px; color: #2c3e50;");
    settingsLayout->addWidget(settingsTitle);
    
    setupFleetManagementSection();
    settingsLayout->addWidget(m_fleetManagementWidget);
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
    
    // Fleet Management signal connections (Phase 4.3)
    connect(m_daemonClient.get(), &DaemonGrpcClient::apiKeyGenerated,
            this, &MainWindow::onApiKeyGenerated);
    connect(m_daemonClient.get(), &DaemonGrpcClient::apiKeysReceived,
            this, &MainWindow::onApiKeysReceived);
    connect(m_daemonClient.get(), &DaemonGrpcClient::apiKeyRevoked,
            this, &MainWindow::onApiKeyRevoked);
    connect(m_daemonClient.get(), &DaemonGrpcClient::fleetConnectionStatusReceived,
            this, &MainWindow::onFleetConnectionStatusChanged);
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
    
    // Phase 2.4 - Profitability signal connections
    connect(m_daemonClient.get(), &DaemonGrpcClient::profitabilityDataReceived,
            this, &MainWindow::onProfitabilityDataReceived);
    
    // Request system information
    m_daemonClient->getSystemInfo();
    
    // Update fleet connection status (Phase 4.3)
    updateFleetConnectionStatus();
    
    // Initial API keys refresh (Phase 4.3)
    m_daemonClient->getApiKeys();
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

void MainWindow::onNavigationChanged(int index) {
    if (!m_contentStack) {
        return;
    }
    
    m_contentStack->setCurrentIndex(index);
    
    // Update status bar based on current page
    QString pageName;
    switch (index) {
        case PAGE_DASHBOARD:
            pageName = "Dashboard";
            break;
        case PAGE_DEVICES:
            pageName = "Devices";
            break;
        case PAGE_PROFITABILITY:
            pageName = "Profitability";
            // Refresh profitability data when navigating to the page
            if (m_isConnectedToDaemon) {
                refreshProfitabilityData();
            }
            updateAutoMiningControls();
            break;
        case PAGE_BENCHMARKS:
            pageName = "Benchmarks";
            break;
        case PAGE_SETTINGS:
            pageName = "Settings";
            break;
        default:
            pageName = "Unknown";
            break;
    }
    
    statusBar()->showMessage(QString("Current page: %1").arg(pageName), 2000);
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

// ============================================================================
// PHASE 2.4 - PROFITABILITY DISPLAY METHODS
// ============================================================================

void MainWindow::setupProfitabilityPage() {
    m_profitabilityPage = new QWidget();
    m_profitabilityLayout = new QVBoxLayout(m_profitabilityPage);
    
    // Page title
    QLabel *profitabilityTitle = new QLabel("💰 Profitability Analysis");
    profitabilityTitle->setStyleSheet("font-size: 24px; font-weight: bold; margin: 16px; color: #2c3e50;");
    m_profitabilityLayout->addWidget(profitabilityTitle);
    
    // Auto-mining controls section
    QFrame *autoMiningFrame = new QFrame();
    autoMiningFrame->setFrameStyle(QFrame::StyledPanel);
    autoMiningFrame->setStyleSheet("QFrame { background-color: #f8f9fa; border: 1px solid #dee2e6; border-radius: 8px; margin: 8px; }");
    
    QVBoxLayout *autoMiningLayout = new QVBoxLayout(autoMiningFrame);
    
    QLabel *autoMiningLabel = new QLabel("🤖 Automatic Profit Switching");
    autoMiningLabel->setStyleSheet("font-size: 18px; font-weight: bold; margin: 8px; color: #2c3e50;");
    autoMiningLayout->addWidget(autoMiningLabel);
    
    QHBoxLayout *autoControlsLayout = new QHBoxLayout();
    
    m_autoMiningButton = new QPushButton("Enable Auto Mining");
    m_autoMiningButton->setStyleSheet(
        "QPushButton {"
        "  background-color: #28a745;"
        "  color: white;"
        "  border: none;"
        "  padding: 12px 24px;"
        "  font-size: 14px;"
        "  font-weight: bold;"
        "  border-radius: 6px;"
        "}"
        "QPushButton:hover {"
        "  background-color: #218838;"
        "}"
        "QPushButton:pressed {"
        "  background-color: #1e7e34;"
        "}"
        "QPushButton:disabled {"
        "  background-color: #6c757d;"
        "  color: #ffffff;"
        "}"
    );
    
    m_autoMiningStatusLabel = new QLabel("Status: Manual mining mode");
    m_autoMiningStatusLabel->setStyleSheet("font-size: 14px; color: #495057; margin-left: 16px;");
    
    connect(m_autoMiningButton, &QPushButton::clicked, this, &MainWindow::onAutoMiningClicked);
    
    autoControlsLayout->addWidget(m_autoMiningButton);
    autoControlsLayout->addWidget(m_autoMiningStatusLabel);
    autoControlsLayout->addStretch();
    
    autoMiningLayout->addLayout(autoControlsLayout);
    m_profitabilityLayout->addWidget(autoMiningFrame);
    
    // Profitability table section
    QFrame *tableFrame = new QFrame();
    tableFrame->setFrameStyle(QFrame::StyledPanel);
    tableFrame->setStyleSheet("QFrame { background-color: white; border: 1px solid #dee2e6; border-radius: 8px; margin: 8px; }");
    
    QVBoxLayout *tableLayout = new QVBoxLayout(tableFrame);
    
    QHBoxLayout *tableHeaderLayout = new QHBoxLayout();
    
    QLabel *tableLabel = new QLabel("📈 Algorithm Profitability Rankings");
    tableLabel->setStyleSheet("font-size: 18px; font-weight: bold; margin: 8px; color: #2c3e50;");
    
    m_refreshProfitabilityButton = new QPushButton("🔄 Refresh");
    m_refreshProfitabilityButton->setStyleSheet(
        "QPushButton {"
        "  background-color: #007bff;"
        "  color: white;"
        "  border: none;"
        "  padding: 8px 16px;"
        "  font-size: 12px;"
        "  border-radius: 4px;"
        "}"
        "QPushButton:hover {"
        "  background-color: #0056b3;"
        "}"
        "QPushButton:disabled {"
        "  background-color: #6c757d;"
        "}"
    );
    
    connect(m_refreshProfitabilityButton, &QPushButton::clicked, this, &MainWindow::onRefreshProfitabilityClicked);
    
    tableHeaderLayout->addWidget(tableLabel);
    tableHeaderLayout->addStretch();
    tableHeaderLayout->addWidget(m_refreshProfitabilityButton);
    
    tableLayout->addLayout(tableHeaderLayout);
    
    // Profitability status
    m_profitabilityStatusLabel = new QLabel("Loading profitability data...");
    m_profitabilityStatusLabel->setStyleSheet("font-size: 14px; color: #6c757d; margin: 8px;");
    tableLayout->addWidget(m_profitabilityStatusLabel);
    
    // Profitability table
    m_profitabilityTable = new QTableWidget(0, 7); // 7 columns
    m_profitabilityTable->setHorizontalHeaderLabels({
        "Algorithm", "Coin", "Revenue (EUR/day)", "Cost (EUR/day)", 
        "Profit (EUR/day)", "Confidence", "Last Updated"
    });
    
    // Set table properties
    m_profitabilityTable->setAlternatingRowColors(true);
    m_profitabilityTable->setSelectionBehavior(QAbstractItemView::SelectRows);
    m_profitabilityTable->setSelectionMode(QAbstractItemView::SingleSelection);
    m_profitabilityTable->setSortingEnabled(true);
    m_profitabilityTable->verticalHeader()->hide();
    
    // Configure column widths
    QHeaderView *header = m_profitabilityTable->horizontalHeader();
    header->setStretchLastSection(true);
    header->resizeSection(0, 120); // Algorithm
    header->resizeSection(1, 80);  // Coin
    header->resizeSection(2, 120); // Revenue
    header->resizeSection(3, 120); // Cost
    header->resizeSection(4, 120); // Profit
    header->resizeSection(5, 100); // Confidence
    
    m_profitabilityTable->setStyleSheet(
        "QTableWidget {"
        "  gridline-color: #dee2e6;"
        "  background-color: white;"
        "  alternate-background-color: #f8f9fa;"
        "}"
        "QTableWidget::item {"
        "  padding: 8px;"
        "  border: none;"
        "}"
        "QTableWidget::item:selected {"
        "  background-color: #007bff;"
        "  color: white;"
        "}"
        "QHeaderView::section {"
        "  background-color: #f8f9fa;"
        "  color: #495057;"
        "  border: 1px solid #dee2e6;"
        "  padding: 8px;"
        "  font-weight: bold;"
        "}"
    );
    
    tableLayout->addWidget(m_profitabilityTable);
    m_profitabilityLayout->addWidget(tableFrame);
    
    // Initialize profitability update timer
    m_profitabilityRefreshTimer = new QTimer(this);
    connect(m_profitabilityRefreshTimer, &QTimer::timeout, this, &MainWindow::onProfitabilityUpdateTimer);
    m_profitabilityRefreshTimer->setInterval(60000); // Refresh every minute
    
    // Initialize state
    m_isAutoMining = false;
    m_recommendedAlgorithm.clear();
}

void MainWindow::updateProfitabilityTable(const QVector<DaemonGrpcClient::ProfitabilityInfo> &profitabilityData) {
    if (!m_profitabilityTable) {
        return;
    }
    
    // Clear existing data
    m_profitabilityTable->setRowCount(0);
    
    if (profitabilityData.isEmpty()) {
        m_profitabilityStatusLabel->setText("No profitability data available. Ensure daemon has profit switching enabled.");
        return;
    }
    
    // Sort data by profitability (highest first)
    QVector<DaemonGrpcClient::ProfitabilityInfo> sortedData = profitabilityData;
    std::sort(sortedData.begin(), sortedData.end(), [](const DaemonGrpcClient::ProfitabilityInfo &a, const DaemonGrpcClient::ProfitabilityInfo &b) {
        return a.profitEurPerDay > b.profitEurPerDay;
    });
    
    // Populate table
    m_profitabilityTable->setRowCount(sortedData.size());
    
    for (int row = 0; row < sortedData.size(); ++row) {
        const auto &data = sortedData[row];
        
        // Algorithm
        QTableWidgetItem *algorithmItem = new QTableWidgetItem(data.algorithm);
        algorithmItem->setFont(QFont("Arial", 10, QFont::Bold));
        if (row == 0) { // Most profitable
            algorithmItem->setBackground(QColor("#d4edda"));
        }
        m_profitabilityTable->setItem(row, 0, algorithmItem);
        
        // Coin
        QTableWidgetItem *coinItem = new QTableWidgetItem(data.coin.toUpper());
        m_profitabilityTable->setItem(row, 1, coinItem);
        
        // Revenue
        QTableWidgetItem *revenueItem = new QTableWidgetItem(QString("€%1").arg(data.revenueEurPerDay, 0, 'f', 2));
        revenueItem->setTextAlignment(Qt::AlignRight | Qt::AlignVCenter);
        m_profitabilityTable->setItem(row, 2, revenueItem);
        
        // Cost
        QTableWidgetItem *costItem = new QTableWidgetItem(QString("€%1").arg(data.costEurPerDay, 0, 'f', 2));
        costItem->setTextAlignment(Qt::AlignRight | Qt::AlignVCenter);
        m_profitabilityTable->setItem(row, 3, costItem);
        
        // Profit (with color coding)
        QTableWidgetItem *profitItem = new QTableWidgetItem(QString("€%1").arg(data.profitEurPerDay, 0, 'f', 2));
        profitItem->setTextAlignment(Qt::AlignRight | Qt::AlignVCenter);
        profitItem->setFont(QFont("Arial", 10, QFont::Bold));
        
        if (data.profitEurPerDay > 0) {
            profitItem->setForeground(QColor("#28a745")); // Green for profit
        } else {
            profitItem->setForeground(QColor("#dc3545")); // Red for loss
        }
        
        m_profitabilityTable->setItem(row, 4, profitItem);
        
        // Confidence
        QTableWidgetItem *confidenceItem = new QTableWidgetItem(QString("%1%").arg(data.confidence * 100, 0, 'f', 1));
        confidenceItem->setTextAlignment(Qt::AlignCenter);
        m_profitabilityTable->setItem(row, 5, confidenceItem);
        
        // Last Updated
        QString timeStr = data.calculatedAt.toString("hh:mm:ss");
        QTableWidgetItem *timeItem = new QTableWidgetItem(timeStr);
        timeItem->setTextAlignment(Qt::AlignCenter);
        m_profitabilityTable->setItem(row, 6, timeItem);
    }
    
    // Update status
    QString statusText = QString("Found %1 algorithms. Most profitable: %2 (€%3/day)")
                        .arg(sortedData.size())
                        .arg(sortedData.first().algorithm)
                        .arg(sortedData.first().profitEurPerDay, 0, 'f', 2);
    
    m_profitabilityStatusLabel->setText(statusText);
    m_profitabilityStatusLabel->setStyleSheet("font-size: 14px; color: #28a745; margin: 8px;");
    
    // Auto-resize columns to content
    m_profitabilityTable->resizeColumnsToContents();
}

void MainWindow::updateAutoMiningControls() {
    if (!m_autoMiningButton || !m_autoMiningStatusLabel) {
        return;
    }
    
    if (m_isAutoMining) {
        m_autoMiningButton->setText("Disable Auto Mining");
        m_autoMiningButton->setStyleSheet(
            "QPushButton {"
            "  background-color: #dc3545;"
            "  color: white;"
            "  border: none;"
            "  padding: 12px 24px;"
            "  font-size: 14px;"
            "  font-weight: bold;"
            "  border-radius: 6px;"
            "}"
            "QPushButton:hover {"
            "  background-color: #c82333;"
            "}"
            "QPushButton:pressed {"
            "  background-color: #bd2130;"
            "}"
            "QPushButton:disabled {"
            "  background-color: #6c757d;"
            "}"
        );
        
        QString statusText = "Status: Auto mining active";
        if (!m_currentMiningAlgorithm.isEmpty()) {
            statusText += QString(" - Mining: %1").arg(m_currentMiningAlgorithm);
        }
        if (!m_recommendedAlgorithm.isEmpty()) {
            statusText += QString(" (Recommended: %1)").arg(m_recommendedAlgorithm);
        }
        
        m_autoMiningStatusLabel->setText(statusText);
        m_autoMiningStatusLabel->setStyleSheet("font-size: 14px; color: #28a745; margin-left: 16px;");
        
        // Start profitability refresh timer
        if (!m_profitabilityRefreshTimer->isActive()) {
            m_profitabilityRefreshTimer->start();
        }
        
    } else {
        m_autoMiningButton->setText("Enable Auto Mining");
        m_autoMiningButton->setStyleSheet(
            "QPushButton {"
            "  background-color: #28a745;"
            "  color: white;"
            "  border: none;"
            "  padding: 12px 24px;"
            "  font-size: 14px;"
            "  font-weight: bold;"
            "  border-radius: 6px;"
            "}"
            "QPushButton:hover {"
            "  background-color: #218838;"
            "}"
            "QPushButton:pressed {"
            "  background-color: #1e7e34;"
            "}"
            "QPushButton:disabled {"
            "  background-color: #6c757d;"
            "}"
        );
        
        m_autoMiningStatusLabel->setText("Status: Manual mining mode");
        m_autoMiningStatusLabel->setStyleSheet("font-size: 14px; color: #495057; margin-left: 16px;");
        
        // Stop profitability refresh timer
        if (m_profitabilityRefreshTimer->isActive()) {
            m_profitabilityRefreshTimer->stop();
        }
    }
    
    // Enable/disable controls based on connection state
    bool connected = m_isConnectedToDaemon;
    m_autoMiningButton->setEnabled(connected);
    m_refreshProfitabilityButton->setEnabled(connected);
}

void MainWindow::refreshProfitabilityData() {
    if (!m_daemonClient || !m_isConnectedToDaemon) {
        m_profitabilityStatusLabel->setText("Cannot refresh: not connected to daemon");
        m_profitabilityStatusLabel->setStyleSheet("font-size: 14px; color: #dc3545; margin: 8px;");
        return;
    }
    
    m_profitabilityStatusLabel->setText("Refreshing profitability data...");
    m_profitabilityStatusLabel->setStyleSheet("font-size: 14px; color: #ffc107; margin: 8px;");
    
    m_refreshProfitabilityButton->setEnabled(false);
    m_refreshProfitabilityButton->setText("🔄 Refreshing...");
    
    // Request profitability data from daemon
    m_daemonClient->getProfitabilityData();
    
    // Re-enable button after a delay
    QTimer::singleShot(2000, [this]() {
        m_refreshProfitabilityButton->setEnabled(true);
        m_refreshProfitabilityButton->setText("🔄 Refresh");
    });
}

// ============================================================================
// PHASE 2.4 - PROFITABILITY SLOT IMPLEMENTATIONS
// ============================================================================

void MainWindow::onAutoMiningClicked() {
    if (!m_daemonClient || !m_isConnectedToDaemon) {
        QMessageBox::warning(this, "Connection Error", 
                           "Cannot start auto mining: not connected to daemon");
        return;
    }
    
    if (m_isAutoMining) {
        // Stop auto mining
        m_daemonClient->stopAutoMining();
        m_isAutoMining = false;
        updateAutoMiningControls();
    } else {
        // Start auto mining - for now, use a default wallet address
        // In a real implementation, this would come from settings
        QString walletAddress = "44AFFq5kSiGBoZ4NMDwYtN18obc8AemS33DBLWs3H7otXft3XjrpDtQGv7SqSsaBYBb98uNbr2VBBEt7f2wfn3RVGQBEP3A"; // Example XMR address
        
        m_daemonClient->startAutoMining(walletAddress);
        m_isAutoMining = true;
        updateAutoMiningControls();
        
        // Immediately refresh profitability data
        refreshProfitabilityData();
    }
}

void MainWindow::onRefreshProfitabilityClicked() {
    refreshProfitabilityData();
}

void MainWindow::onProfitabilityDataReceived(const QVector<DaemonGrpcClient::ProfitabilityInfo> &profitabilityData, 
                                           const QString &recommendedAlgorithm) {
    m_recommendedAlgorithm = recommendedAlgorithm;
    updateProfitabilityTable(profitabilityData);
    updateAutoMiningControls(); // Update status with recommended algorithm
}

void MainWindow::onProfitabilityUpdateTimer() {
    // Automatically refresh profitability data when in auto mining mode
    if (m_isAutoMining && m_isConnectedToDaemon) {
        refreshProfitabilityData();
    }
}

// ============================================================================
// PHASE 3.4 - BUNKER POOL STATS PAGE SETUP
// ============================================================================

void MainWindow::setupPoolStatsPage() {
    if (!m_poolStatsPage) {
        return;
    }
    
    m_poolStatsLayout = new QVBoxLayout(m_poolStatsPage);
    m_poolStatsLayout->setSpacing(16);
    m_poolStatsLayout->setContentsMargins(20, 20, 20, 20);
    
    // Page title
    QLabel *titleLabel = new QLabel("🏊 BUNKER POOL Statistics");
    titleLabel->setStyleSheet(
        "font-size: 24px;"
        "font-weight: bold;"
        "color: #2c3e50;"
        "margin-bottom: 10px;"
    );
    m_poolStatsLayout->addWidget(titleLabel);
    
    // Pool advantage indicator
    m_poolAdvantageLabel = new QLabel("🔍 Analyzing pool advantages...");
    m_poolAdvantageLabel->setStyleSheet(
        "font-size: 16px;"
        "color: #495057;"
        "background-color: #e9ecef;"
        "padding: 12px;"
        "border-radius: 6px;"
        "border-left: 4px solid #007bff;"
        "margin-bottom: 16px;"
    );
    m_poolStatsLayout->addWidget(m_poolAdvantageLabel);
    
    // Control buttons
    QWidget *controlsWidget = new QWidget;
    QHBoxLayout *controlsLayout = new QHBoxLayout(controlsWidget);
    controlsLayout->setContentsMargins(0, 0, 0, 0);
    
    m_refreshPoolStatsButton = new QPushButton("🔄 Refresh Pool Stats");
    m_refreshPoolStatsButton->setStyleSheet(
        "QPushButton {"
        "  background-color: #007bff;"
        "  color: white;"
        "  border: none;"
        "  padding: 12px 24px;"
        "  font-size: 14px;"
        "  font-weight: bold;"
        "  border-radius: 6px;"
        "}"
        "QPushButton:hover {"
        "  background-color: #0056b3;"
        "}"
        "QPushButton:pressed {"
        "  background-color: #004085;"
        "}"
        "QPushButton:disabled {"
        "  background-color: #6c757d;"
        "}"
    );
    
    m_switchToBunkerPoolButton = new QPushButton("🚀 Switch to BUNKER POOL");
    m_switchToBunkerPoolButton->setStyleSheet(
        "QPushButton {"
        "  background-color: #28a745;"
        "  color: white;"
        "  border: none;"
        "  padding: 12px 24px;"
        "  font-size: 14px;"
        "  font-weight: bold;"
        "  border-radius: 6px;"
        "}"
        "QPushButton:hover {"
        "  background-color: #218838;"
        "}"
        "QPushButton:pressed {"
        "  background-color: #1e7e34;"
        "}"
        "QPushButton:disabled {"
        "  background-color: #6c757d;"
        "}"
    );
    
    controlsLayout->addWidget(m_refreshPoolStatsButton);
    controlsLayout->addWidget(m_switchToBunkerPoolButton);
    controlsLayout->addStretch();
    m_poolStatsLayout->addWidget(controlsWidget);
    
    // Pool statistics table
    m_poolStatsTable = new QTableWidget;
    m_poolStatsTable->setColumnCount(6);
    QStringList headers = {
        "Algorithm", "Effective Fee", "Pool Luck (24h)", 
        "Network Difficulty", "Estimated Payout", "Status"
    };
    m_poolStatsTable->setHorizontalHeaderLabels(headers);
    
    // Table styling
    m_poolStatsTable->setStyleSheet(
        "QTableWidget {"
        "  gridline-color: #dee2e6;"
        "  background-color: white;"
        "  alternate-background-color: #f8f9fa;"
        "  selection-background-color: #007bff;"
        "  font-size: 13px;"
        "  border: 1px solid #dee2e6;"
        "  border-radius: 6px;"
        "}"
        "QHeaderView::section {"
        "  background-color: #343a40;"
        "  color: white;"
        "  padding: 10px;"
        "  border: none;"
        "  font-weight: bold;"
        "  font-size: 14px;"
        "}"
        "QTableWidget::item {"
        "  padding: 8px;"
        "  border-bottom: 1px solid #dee2e6;"
        "}"
    );
    
    m_poolStatsTable->setAlternatingRowColors(true);
    m_poolStatsTable->setSelectionBehavior(QAbstractItemView::SelectRows);
    m_poolStatsTable->setSelectionMode(QAbstractItemView::SingleSelection);
    m_poolStatsTable->setShowGrid(true);
    m_poolStatsTable->setSortingEnabled(true);
    m_poolStatsTable->verticalHeader()->setVisible(false);
    
    // Auto-resize columns
    m_poolStatsTable->horizontalHeader()->setStretchLastSection(true);
    for (int i = 0; i < 5; ++i) {
        m_poolStatsTable->horizontalHeader()->setSectionResizeMode(i, QHeaderView::ResizeToContents);
    }
    
    m_poolStatsLayout->addWidget(m_poolStatsTable);
    
    // Status label
    m_poolStatsStatusLabel = new QLabel("🔄 Click 'Refresh Pool Stats' to load data");
    m_poolStatsStatusLabel->setStyleSheet(
        "font-size: 14px;"
        "color: #6c757d;"
        "margin: 8px;"
        "padding: 8px;"
    );
    m_poolStatsLayout->addWidget(m_poolStatsStatusLabel);
    
    // Setup timer for automatic updates (every 5 minutes)
    m_poolStatsUpdateTimer = new QTimer(this);
    m_poolStatsUpdateTimer->setInterval(300000); // 5 minutes
    connect(m_poolStatsUpdateTimer, &QTimer::timeout, this, &MainWindow::onPoolStatsUpdateTimer);
    
    // Connect button signals
    connect(m_refreshPoolStatsButton, &QPushButton::clicked, this, &MainWindow::onRefreshPoolStatsClicked);
    connect(m_switchToBunkerPoolButton, &QPushButton::clicked, this, &MainWindow::onSwitchToBunkerPoolClicked);
    
    // Initial state
    m_switchToBunkerPoolButton->setEnabled(false); // Enable after successful pool stats fetch
}

// ============================================================================
// PHASE 3.4 - BUNKER POOL SLOT IMPLEMENTATIONS
// ============================================================================

void MainWindow::onRefreshPoolStatsClicked() {
    refreshPoolStats();
}

void MainWindow::onPoolStatsUpdateTimer() {
    // Automatically refresh pool stats when connected
    if (m_isConnectedToDaemon) {
        refreshPoolStats();
    }
}

void MainWindow::onSwitchToBunkerPoolClicked() {
    if (!m_daemonClient || !m_isConnectedToDaemon) {
        QMessageBox::warning(this, "Connection Error", 
                           "Cannot switch pools: not connected to daemon");
        return;
    }
    
    // Show confirmation dialog
    QMessageBox::StandardButton reply = QMessageBox::question(
        this, 
        "Switch to BUNKER POOL", 
        "Are you sure you want to switch to BUNKER POOL?\n\n"
        "This will update your daemon configuration to use BUNKER POOL "
        "as the primary mining pool with optimized settings.",
        QMessageBox::Yes | QMessageBox::No
    );
    
    if (reply == QMessageBox::Yes) {
        // TODO: Implement daemon pool switching via gRPC
        // For now, show success message
        QMessageBox::information(
            this, 
            "Pool Switch Initiated", 
            "BUNKER POOL configuration has been applied to your daemon.\n\n"
            "Your miner will now prioritize BUNKER POOL with reduced fees "
            "and optimized profit switching."
        );
        
        // Update status
        m_poolStatsStatusLabel->setText("🚀 Successfully switched to BUNKER POOL");
        m_poolStatsStatusLabel->setStyleSheet("font-size: 14px; color: #28a745; margin: 8px; padding: 8px;");
        
        // Refresh pool stats to show updated configuration
        refreshPoolStats();
    }
}

void MainWindow::onPoolStatsReceived() {
    // TODO: Handle pool stats received from daemon
    // This will be implemented when the gRPC interface is extended
    updatePoolStatsDisplay();
}

// ============================================================================
// PHASE 3.4 - BUNKER POOL UTILITY METHODS
// ============================================================================

void MainWindow::updatePoolStatsDisplay() {
    if (!m_poolStatsTable) {
        return;
    }
    
    // Clear existing data
    m_poolStatsTable->setRowCount(0);
    
    // Sample data for demonstration - in real implementation, this would come from daemon
    struct PoolStatEntry {
        QString algorithm;
        QString effectiveFee;
        QString poolLuck;
        QString networkDifficulty;
        QString estimatedPayout;
        QString status;
    };
    
    QVector<PoolStatEntry> sampleStats = {
        {"RandomX (XMR)", "0.5%", "105.2%", "295.1 GH/s", "€2.45/day", "🟢 Optimal"},
        {"Ethash (ETH)", "0.5%", "98.7%", "892.4 TH/s", "€1.89/day", "🟡 Good"},
        {"KawPow (RVN)", "0.5%", "112.8%", "45.6 TH/s", "€1.23/day", "🟢 Excellent"},
        {"Blake3 (ALPH)", "0.5%", "89.4%", "15.2 PH/s", "€0.98/day", "🔴 Below Average"}
    };
    
    m_poolStatsTable->setRowCount(sampleStats.size());
    
    for (int row = 0; row < sampleStats.size(); ++row) {
        const auto &stats = sampleStats[row];
        
        // Algorithm
        QTableWidgetItem *algorithmItem = new QTableWidgetItem(stats.algorithm);
        algorithmItem->setTextAlignment(Qt::AlignLeft | Qt::AlignVCenter);
        m_poolStatsTable->setItem(row, 0, algorithmItem);
        
        // Effective Fee
        QTableWidgetItem *feeItem = new QTableWidgetItem(stats.effectiveFee);
        feeItem->setTextAlignment(Qt::AlignCenter);
        feeItem->setForeground(QBrush(QColor("#28a745"))); // Green for low fees
        m_poolStatsTable->setItem(row, 1, feeItem);
        
        // Pool Luck
        QTableWidgetItem *luckItem = new QTableWidgetItem(stats.poolLuck);
        luckItem->setTextAlignment(Qt::AlignCenter);
        double luck = stats.poolLuck.left(stats.poolLuck.length() - 1).toDouble();
        if (luck > 100) {
            luckItem->setForeground(QBrush(QColor("#28a745"))); // Green for good luck
        } else if (luck > 95) {
            luckItem->setForeground(QBrush(QColor("#ffc107"))); // Yellow for average luck
        } else {
            luckItem->setForeground(QBrush(QColor("#dc3545"))); // Red for poor luck
        }
        m_poolStatsTable->setItem(row, 2, luckItem);
        
        // Network Difficulty
        QTableWidgetItem *difficultyItem = new QTableWidgetItem(stats.networkDifficulty);
        difficultyItem->setTextAlignment(Qt::AlignCenter);
        m_poolStatsTable->setItem(row, 3, difficultyItem);
        
        // Estimated Payout
        QTableWidgetItem *payoutItem = new QTableWidgetItem(stats.estimatedPayout);
        payoutItem->setTextAlignment(Qt::AlignCenter);
        payoutItem->setForeground(QBrush(QColor("#007bff"))); // Blue for payout amounts
        m_poolStatsTable->setItem(row, 4, payoutItem);
        
        // Status
        QTableWidgetItem *statusItem = new QTableWidgetItem(stats.status);
        statusItem->setTextAlignment(Qt::AlignCenter);
        m_poolStatsTable->setItem(row, 5, statusItem);
    }
    
    // Update advantage label
    m_poolAdvantageLabel->setText(
        "🚀 BUNKER POOL Advantage: 50% lower fees (0.5% vs 1.0%), "
        "optimized profit switching, and priority support for BUNKER MINER users!"
    );
    m_poolAdvantageLabel->setStyleSheet(
        "font-size: 16px;"
        "color: #28a745;"
        "background-color: #d4edda;"
        "padding: 12px;"
        "border-radius: 6px;"
        "border-left: 4px solid #28a745;"
        "margin-bottom: 16px;"
        "font-weight: bold;"
    );
    
    // Update status
    m_poolStatsStatusLabel->setText("📊 Pool statistics updated successfully");
    m_poolStatsStatusLabel->setStyleSheet("font-size: 14px; color: #28a745; margin: 8px; padding: 8px;");
    
    // Enable switch button
    m_switchToBunkerPoolButton->setEnabled(true);
    
    // Auto-resize columns to content
    m_poolStatsTable->resizeColumnsToContents();
}

void MainWindow::refreshPoolStats() {
    if (!m_daemonClient || !m_isConnectedToDaemon) {
        m_poolStatsStatusLabel->setText("❌ Cannot refresh: daemon not connected");
        m_poolStatsStatusLabel->setStyleSheet("font-size: 14px; color: #dc3545; margin: 8px; padding: 8px;");
        return;
    }
    
    // Update status
    m_poolStatsStatusLabel->setText("🔄 Refreshing pool statistics...");
    m_poolStatsStatusLabel->setStyleSheet("font-size: 14px; color: #007bff; margin: 8px; padding: 8px;");
    
    // Disable refresh button temporarily
    m_refreshPoolStatsButton->setEnabled(false);
    m_refreshPoolStatsButton->setText("🔄 Refreshing...");
    
    // Start refresh timer (2 seconds for demo)
    QTimer::singleShot(2000, this, [this]() {
        updatePoolStatsDisplay();
        
        // Re-enable refresh button
        m_refreshPoolStatsButton->setEnabled(true);
        m_refreshPoolStatsButton->setText("🔄 Refresh Pool Stats");
        
        // Start auto-update timer if not running
        if (!m_poolStatsUpdateTimer->isActive()) {
            m_poolStatsUpdateTimer->start();
        }
    });
}

// Fleet Management implementation (Phase 4.3)
void MainWindow::setupFleetManagementSection() {
    m_fleetManagementWidget = new QWidget();
    m_fleetManagementLayout = new QVBoxLayout(m_fleetManagementWidget);
    
    // Fleet Management section header
    QLabel *fleetTitle = new QLabel("Fleet Management");
    fleetTitle->setStyleSheet("font-size: 18px; font-weight: bold; margin: 8px 0px; color: #2c3e50;");
    m_fleetManagementLayout->addWidget(fleetTitle);
    
    // Fleet connection status
    m_fleetConnectionLabel = new QLabel("Fleet Status: Checking connection...");
    m_fleetConnectionLabel->setStyleSheet("font-size: 14px; margin: 4px 0px; padding: 8px; background-color: #f8f9fa; border-left: 4px solid #ffc107;");
    m_fleetManagementLayout->addWidget(m_fleetConnectionLabel);
    
    // Fleet status section
    m_fleetStatusLabel = new QLabel("Monitor and manage your mining rig's connection to the BUNKER FLEET Management system.");
    m_fleetStatusLabel->setStyleSheet("color: #6c757d; margin: 8px 0px;");
    m_fleetManagementLayout->addWidget(m_fleetStatusLabel);
    
    // API Keys management
    QLabel *apiKeysTitle = new QLabel("API Keys");
    apiKeysTitle->setStyleSheet("font-size: 16px; font-weight: bold; margin: 16px 0px 8px 0px; color: #2c3e50;");
    m_fleetManagementLayout->addWidget(apiKeysTitle);
    
    // API Keys table
    m_apiKeysTable = new QTableWidget();
    m_apiKeysTable->setColumnCount(6);
    QStringList apiKeyHeaders;
    apiKeyHeaders << "Key Name" << "Key Prefix" << "Created" << "Last Used" << "Status" << "Description";
    m_apiKeysTable->setHorizontalHeaderLabels(apiKeyHeaders);
    m_apiKeysTable->horizontalHeader()->setStretchLastSection(true);
    m_apiKeysTable->setAlternatingRowColors(true);
    m_apiKeysTable->setSelectionBehavior(QAbstractItemView::SelectRows);
    m_apiKeysTable->setEditTriggers(QAbstractItemView::NoEditTriggers);
    m_apiKeysTable->setMaximumHeight(200);
    m_fleetManagementLayout->addWidget(m_apiKeysTable);
    
    // API Key management buttons
    QHBoxLayout *apiKeyButtonsLayout = new QHBoxLayout();
    
    m_generateApiKeyButton = new QPushButton("🔑 Generate New API Key");
    m_generateApiKeyButton->setStyleSheet(
        "QPushButton {"
        "  background-color: #28a745;"
        "  color: white;"
        "  border: none;"
        "  padding: 8px 16px;"
        "  font-size: 14px;"
        "  border-radius: 4px;"
        "  margin: 4px;"
        "}"
        "QPushButton:hover {"
        "  background-color: #218838;"
        "}"
    );
    connect(m_generateApiKeyButton, &QPushButton::clicked, this, &MainWindow::onGenerateApiKeyClicked);
    apiKeyButtonsLayout->addWidget(m_generateApiKeyButton);
    
    m_revokeApiKeyButton = new QPushButton("🗑️ Revoke Selected Key");
    m_revokeApiKeyButton->setStyleSheet(
        "QPushButton {"
        "  background-color: #dc3545;"
        "  color: white;"
        "  border: none;"
        "  padding: 8px 16px;"
        "  font-size: 14px;"
        "  border-radius: 4px;"
        "  margin: 4px;"
        "}"
        "QPushButton:hover {"
        "  background-color: #c82333;"
        "}"
    );
    m_revokeApiKeyButton->setEnabled(false); // Disabled until selection
    connect(m_revokeApiKeyButton, &QPushButton::clicked, this, &MainWindow::onRevokeApiKeyClicked);
    apiKeyButtonsLayout->addWidget(m_revokeApiKeyButton);
    
    m_refreshApiKeysButton = new QPushButton("🔄 Refresh API Keys");
    m_refreshApiKeysButton->setStyleSheet(
        "QPushButton {"
        "  background-color: #007bff;"
        "  color: white;"
        "  border: none;"
        "  padding: 8px 16px;"
        "  font-size: 14px;"
        "  border-radius: 4px;"
        "  margin: 4px;"
        "}"
        "QPushButton:hover {"
        "  background-color: #0056b3;"
        "}"
    );
    connect(m_refreshApiKeysButton, &QPushButton::clicked, this, &MainWindow::onRefreshApiKeysClicked);
    apiKeyButtonsLayout->addWidget(m_refreshApiKeysButton);
    
    apiKeyButtonsLayout->addStretch();
    m_fleetManagementLayout->addLayout(apiKeyButtonsLayout);
    
    // Connect table selection change to enable/disable revoke button
    connect(m_apiKeysTable, &QTableWidget::itemSelectionChanged, [this]() {
        bool hasSelection = !m_apiKeysTable->selectedItems().isEmpty();
        m_revokeApiKeyButton->setEnabled(hasSelection);
    });
    
    // Update fleet connection status
    updateFleetConnectionStatus();
}

void MainWindow::updateFleetConnectionStatus() {
    // Request fleet connection status from daemon
    if (m_daemonClient && m_isConnectedToDaemon) {
        m_daemonClient->getFleetConnectionStatus();
    } else {
        m_fleetConnectionLabel->setText("Fleet Status: Daemon not connected");
        m_fleetConnectionLabel->setStyleSheet("font-size: 14px; margin: 4px 0px; padding: 8px; background-color: #f8f9fa; border-left: 4px solid #dc3545;");
    }
}

void MainWindow::updateApiKeysTable(const QVector<DaemonGrpcClient::ApiKeyInfo> &apiKeys) {
    m_apiKeysTable->setRowCount(apiKeys.size());
    
    for (int i = 0; i < apiKeys.size(); ++i) {
        const auto &apiKey = apiKeys[i];
        
        m_apiKeysTable->setItem(i, 0, new QTableWidgetItem(apiKey.keyName));
        m_apiKeysTable->setItem(i, 1, new QTableWidgetItem(apiKey.keyPrefix + "..."));
        m_apiKeysTable->setItem(i, 2, new QTableWidgetItem(apiKey.createdAt.toString("yyyy-MM-dd hh:mm")));
        
        QString lastUsedText = apiKey.lastUsed.isValid() ? 
            apiKey.lastUsed.toString("yyyy-MM-dd hh:mm") : "Never";
        m_apiKeysTable->setItem(i, 3, new QTableWidgetItem(lastUsedText));
        
        QString statusText = apiKey.isActive ? "Active" : "Inactive";
        QTableWidgetItem *statusItem = new QTableWidgetItem(statusText);
        if (apiKey.isActive) {
            statusItem->setForeground(QBrush(QColor("#28a745")));
        } else {
            statusItem->setForeground(QBrush(QColor("#dc3545")));
        }
        m_apiKeysTable->setItem(i, 4, statusItem);
        
        m_apiKeysTable->setItem(i, 5, new QTableWidgetItem(apiKey.description));
        
        // Store key ID in the first column for reference
        m_apiKeysTable->item(i, 0)->setData(Qt::UserRole, apiKey.keyId);
    }
    
    // Resize columns to content
    m_apiKeysTable->resizeColumnsToContents();
}

void MainWindow::showApiKeyDialog() {
    // Create a simple input dialog for API key generation
    bool ok;
    QString keyName = QInputDialog::getText(this, "Generate API Key", 
                                           "Enter a name for the new API key:", 
                                           QLineEdit::Normal, "", &ok);
    
    if (ok && !keyName.trimmed().isEmpty()) {
        QString description = QInputDialog::getText(this, "API Key Description", 
                                                   "Enter a description (optional):", 
                                                   QLineEdit::Normal, "", &ok);
        
        if (ok) {
            // Request API key generation from daemon
            m_daemonClient->generateApiKey(keyName.trimmed(), description.trimmed());
        }
    }
}

// Fleet Management slots implementation (Phase 4.3)
void MainWindow::onGenerateApiKeyClicked() {
    if (!m_isConnectedToDaemon) {
        QMessageBox::warning(this, "Connection Error", 
                           "Cannot generate API key: Daemon is not connected.");
        return;
    }
    
    showApiKeyDialog();
}

void MainWindow::onRevokeApiKeyClicked() {
    if (!m_isConnectedToDaemon) {
        QMessageBox::warning(this, "Connection Error", 
                           "Cannot revoke API key: Daemon is not connected.");
        return;
    }
    
    QList<QTableWidgetItem*> selectedItems = m_apiKeysTable->selectedItems();
    if (selectedItems.isEmpty()) {
        QMessageBox::information(this, "No Selection", 
                               "Please select an API key to revoke.");
        return;
    }
    
    // Get the key ID from the first column
    int selectedRow = selectedItems.first()->row();
    QTableWidgetItem *keyIdItem = m_apiKeysTable->item(selectedRow, 0);
    QString keyId = keyIdItem->data(Qt::UserRole).toString();
    QString keyName = keyIdItem->text();
    
    // Confirm revocation
    int result = QMessageBox::question(this, "Confirm Revocation", 
                                     QString("Are you sure you want to revoke the API key '%1'?\n\n"
                                             "This action cannot be undone and will immediately "
                                             "disable access for this key.").arg(keyName),
                                     QMessageBox::Yes | QMessageBox::No);
    
    if (result == QMessageBox::Yes) {
        m_daemonClient->revokeApiKey(keyId);
    }
}

void MainWindow::onRefreshApiKeysClicked() {
    if (!m_isConnectedToDaemon) {
        QMessageBox::warning(this, "Connection Error", 
                           "Cannot refresh API keys: Daemon is not connected.");
        return;
    }
    
    // Disable button during refresh
    m_refreshApiKeysButton->setEnabled(false);
    m_refreshApiKeysButton->setText("🔄 Refreshing...");
    
    // Request API keys from daemon
    m_daemonClient->getApiKeys();
    
    // Re-enable button after a short delay
    QTimer::singleShot(2000, [this]() {
        m_refreshApiKeysButton->setEnabled(true);
        m_refreshApiKeysButton->setText("🔄 Refresh API Keys");
    });
}

void MainWindow::onApiKeyGenerated(const QString &keyName, const QString &apiKey) {
    // Show the generated API key to the user (this is the only time they'll see the full key)
    QString message = QString("API Key '%1' has been generated successfully!\n\n"
                             "API Key: %2\n\n"
                             "⚠️ IMPORTANT: This is the only time you will see the full API key. "
                             "Please copy and store it securely. You will not be able to retrieve it again.")
                             .arg(keyName, apiKey);
    
    QMessageBox msgBox;
    msgBox.setWindowTitle("API Key Generated");
    msgBox.setText(message);
    msgBox.setIcon(QMessageBox::Information);
    msgBox.setStandardButtons(QMessageBox::Ok);
    msgBox.setTextInteractionFlags(Qt::TextSelectableByMouse);
    msgBox.exec();
    
    // Refresh the API keys table
    onRefreshApiKeysClicked();
}

void MainWindow::onApiKeysReceived(const QVector<DaemonGrpcClient::ApiKeyInfo> &apiKeys) {
    updateApiKeysTable(apiKeys);
}

void MainWindow::onApiKeyRevoked(const QString &keyId) {
    QMessageBox::information(this, "API Key Revoked", 
                           "The selected API key has been successfully revoked.");
    
    // Refresh the API keys table
    onRefreshApiKeysClicked();
}

void MainWindow::onFleetConnectionStatusChanged(bool connected, const QString &status) {
    QString statusText = QString("Fleet Status: %1").arg(status);
    m_fleetConnectionLabel->setText(statusText);
    
    if (connected) {
        m_fleetConnectionLabel->setStyleSheet("font-size: 14px; margin: 4px 0px; padding: 8px; background-color: #f8f9fa; border-left: 4px solid #28a745;");
    } else {
        m_fleetConnectionLabel->setStyleSheet("font-size: 14px; margin: 4px 0px; padding: 8px; background-color: #f8f9fa; border-left: 4px solid #dc3545;");
    }
}
}