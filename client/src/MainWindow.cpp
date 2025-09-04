#include "MainWindow.h"
#include <QMessageBox>

/**
 * MainWindow implementation for BUNKER MINER Client
 * Phase 0.1: Stub implementation to validate build system
 */

MainWindow::MainWindow(QWidget *parent)
    : QMainWindow(parent)
    , m_centralWidget(nullptr)
    , m_mainLayout(nullptr)
    , m_titleLabel(nullptr)
    , m_statusLabel(nullptr)
    , m_connectButton(nullptr)
    , m_startButton(nullptr)
    , m_stopButton(nullptr)
{
    setupUI();
}

void MainWindow::setupUI() {
    setWindowTitle("BUNKER MINER Client v0.1.0");
    setMinimumSize(800, 600);
    
    m_centralWidget = new QWidget(this);
    setCentralWidget(m_centralWidget);
    
    m_mainLayout = new QVBoxLayout(m_centralWidget);
    
    // Title
    m_titleLabel = new QLabel("BUNKER MINER Client", this);
    m_titleLabel->setStyleSheet("font-size: 20px; font-weight: bold; margin: 10px;");
    m_mainLayout->addWidget(m_titleLabel);
    
    // Status
    m_statusLabel = new QLabel("Status: Disconnected (Phase 0.1 Stub)", this);
    m_statusLabel->setStyleSheet("margin: 5px;");
    m_mainLayout->addWidget(m_statusLabel);
    
    // Connect button
    m_connectButton = new QPushButton("Connect to Daemon", this);
    connect(m_connectButton, &QPushButton::clicked, this, &MainWindow::onConnectDaemon);
    m_mainLayout->addWidget(m_connectButton);
    
    // Mining controls (disabled in stub)
    m_startButton = new QPushButton("Start Mining", this);
    m_startButton->setEnabled(false);
    connect(m_startButton, &QPushButton::clicked, this, &MainWindow::onStartMining);
    m_mainLayout->addWidget(m_startButton);
    
    m_stopButton = new QPushButton("Stop Mining", this);
    m_stopButton->setEnabled(false);
    connect(m_stopButton, &QPushButton::clicked, this, &MainWindow::onStopMining);
    m_mainLayout->addWidget(m_stopButton);
    
    m_mainLayout->addStretch();
}

void MainWindow::onConnectDaemon() {
    QMessageBox::information(this, "Phase 0.1 Stub", 
        "Daemon connection will be implemented in Phase 2.1\n"
        "This validates the Qt build system is working correctly.");
}

void MainWindow::onStartMining() {
    QMessageBox::information(this, "Phase 0.1 Stub",
        "Mining controls will be implemented in Phase 2.2");
}

void MainWindow::onStopMining() {
    QMessageBox::information(this, "Phase 0.1 Stub",
        "Mining controls will be implemented in Phase 2.2");
}

void MainWindow::updateStatus(const QString &status) {
    if (m_statusLabel) {
        m_statusLabel->setText(QString("Status: %1").arg(status));
    }
}