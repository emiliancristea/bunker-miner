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

// Forward declaration
class DaemonGrpcClient;

/**
 * MainWindow class for BUNKER MINER Client - Phase 2.1
 * 
 * Provides the main application shell with navigation and daemon integration.
 * Features:
 * - Navigation sidebar with Dashboard, Devices, Benchmarks, Settings sections
 * - Real-time daemon connection status
 * - System information display from daemon
 * - Error handling for disconnected states
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
    
    // Status bar
    QLabel *m_statusBarLabel;
    
    // Daemon client
    std::unique_ptr<DaemonGrpcClient> m_daemonClient;
    
    // State tracking
    bool m_isConnectedToDaemon;
    enum NavigationPage {
        PAGE_DASHBOARD = 0,
        PAGE_DEVICES = 1,
        PAGE_BENCHMARKS = 2,
        PAGE_SETTINGS = 3
    };
};