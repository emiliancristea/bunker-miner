#pragma once

#include <QMainWindow>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QLabel>
#include <QPushButton>
#include <QWidget>
#include <QStackedWidget>
#include <QListWidget>
#include <QTreeWidget>
#include <QTextEdit>
#include <QStatusBar>
#include <memory>
#include <QMap>

// Forward declarations
class DaemonGrpcClient;
class DeviceTelemetryWidget;
class TelemetryWorker;

/**
 * MainWindow class for BUNKER MINER Client - Phase 2.2
 * 
 * Enhanced main application window with real-time telemetry and mining controls.
 * 
 * Phase 2.2 Features:
 * - Real-time telemetry display with device-specific widgets
 * - Start/Stop mining controls with state management
 * - Live dashboard updates with comprehensive device metrics
 * - Thread-safe telemetry data integration
 * - Mining operation feedback and error handling
 * 
 * UI Components:
 * - Navigation sidebar with Dashboard, Devices, Benchmarks, Settings sections
 * - Real-time daemon connection status with health monitoring
 * - Dynamic device telemetry widgets with live updates
 * - Mining control buttons with state validation
 * - Comprehensive error handling for all operation states
 */
class MainWindow : public QMainWindow {
    Q_OBJECT

public:
    explicit MainWindow(QWidget *parent = nullptr);
    ~MainWindow() override = default;

private slots:
    // Navigation slots
    void onNavigationChanged(int index);
    
    // Daemon connection slots
    void onDaemonConnected();
    void onDaemonDisconnected();
    void onDaemonConnectionError(const QString &error);
    void onSystemInfoReceived(const std::vector<DaemonGrpcClient::DeviceInfo> &devices, 
                              const DaemonGrpcClient::SystemInfo &systemInfo,
                              const DaemonGrpcClient::VersionInfo &versionInfo);
    void onRefreshSystemInfo();
    
    // Mining control slots (Phase 2.2)
    void onStartMiningClicked();
    void onStopMiningClicked();
    void onMiningStarted(const QString &algorithm);
    void onMiningStopped();
    void onMiningError(const QString &error);
    
    // Telemetry slots (Phase 2.2)
    void onTelemetryReceived(const TelemetryWorker::TelemetryData &telemetryData);
    void onTelemetryStreamStarted();
    void onTelemetryStreamStopped();
    void onTelemetryStreamError(const QString &error);
    
    // Device interaction slots
    void onDeviceClicked(const QString &deviceId);

private:
    void setupUI();
    void setupNavigationSidebar();
    void setupMainContent();
    void setupStatusBar();
    void initializeDaemonClient();
    
    void updateConnectionStatus(const QString &status, bool connected);
    void populateSystemInfo(const std::vector<DaemonGrpcClient::DeviceInfo> &devices = {}, 
                            const DaemonGrpcClient::SystemInfo &systemInfo = {},
                            const DaemonGrpcClient::VersionInfo &versionInfo = {});
    void showErrorState(const QString &message);
    void showConnectedState();
    
    // Mining control methods (Phase 2.2)
    void setupMiningControls();
    void updateMiningControlsState(bool miningActive);
    void showMiningStatus(const QString &status, bool isError = false);
    
    // Telemetry display methods (Phase 2.2)
    void setupTelemetryDisplay();
    void createDeviceWidgets(const std::vector<DaemonGrpcClient::DeviceInfo> &devices);
    void clearDeviceWidgets();
    void updateTelemetryDisplay();
    
    // UI Layout Components
    QWidget *m_centralWidget;
    QHBoxLayout *m_mainLayout;
    
    // Navigation sidebar
    QWidget *m_sidebarWidget;
    QVBoxLayout *m_sidebarLayout;
    QListWidget *m_navigationList;
    QLabel *m_connectionStatusLabel;
    QPushButton *m_refreshButton;
    
    // Main content area
    QStackedWidget *m_contentStack;
    
    // Page widgets
    QWidget *m_dashboardPage;
    QWidget *m_devicesPage;
    QWidget *m_benchmarksPage;
    QWidget *m_settingsPage;
    
    // Dashboard content
    QVBoxLayout *m_dashboardLayout;
    QLabel *m_daemonStatusLabel;
    QTreeWidget *m_systemInfoTree;
    QTextEdit *m_logOutput;
    
    // Devices page content
    QVBoxLayout *m_devicesLayout;
    QTreeWidget *m_devicesTree;
    
    // Mining controls (Phase 2.2)
    QWidget *m_miningControlsWidget;
    QHBoxLayout *m_miningControlsLayout;
    QPushButton *m_startMiningButton;
    QPushButton *m_stopMiningButton;
    QLabel *m_miningStatusLabel;
    QLabel *m_algorithmStatusLabel;
    
    // Telemetry display (Phase 2.2)
    QWidget *m_telemetryDisplayWidget;
    QVBoxLayout *m_telemetryDisplayLayout;
    QScrollArea *m_deviceScrollArea;
    QWidget *m_deviceGridWidget;
    QGridLayout *m_deviceGridLayout;
    QMap<QString, DeviceTelemetryWidget*> m_deviceWidgets;
    QLabel *m_telemetryStatusLabel;
    
    // Status bar
    QLabel *m_statusBarLabel;
    
    // Daemon client
    std::unique_ptr<DaemonGrpcClient> m_daemonClient;
    
    // State tracking
    bool m_isConnectedToDaemon;
    bool m_isMining; // Phase 2.2
    QString m_currentMiningAlgorithm; // Phase 2.2
    std::vector<DaemonGrpcClient::DeviceInfo> m_lastDeviceInfo; // Phase 2.2
    
    enum NavigationPage {
        PAGE_DASHBOARD = 0,
        PAGE_DEVICES = 1,
        PAGE_BENCHMARKS = 2,
        PAGE_SETTINGS = 3
    };
};