#include <QApplication>
#include <QMainWindow>
#include <QVBoxLayout>
#include <QLabel>
#include <QPushButton>
#include <QWidget>
#include <QMessageBox>
#include <iostream>

/**
 * BUNKER MINER Client - Phase 0.1 Stub Implementation
 * 
 * This is a basic Qt application stub that demonstrates the client can be built
 * and run successfully. Full implementation will be added in Phase 2.
 */

class BunkerMinerMainWindow : public QMainWindow {
    Q_OBJECT

public:
    BunkerMinerMainWindow(QWidget *parent = nullptr) : QMainWindow(parent) {
        setupUI();
    }

private slots:
    void onConnectClicked() {
        QMessageBox::information(this, "BUNKER MINER", 
            "Daemon connection functionality will be implemented in Phase 2.1\n"
            "This stub confirms the Qt client builds successfully.");
    }

private:
    void setupUI() {
        setWindowTitle("BUNKER MINER Client v0.1.0");
        setMinimumSize(600, 400);
        
        auto *centralWidget = new QWidget(this);
        setCentralWidget(centralWidget);
        
        auto *layout = new QVBoxLayout(centralWidget);
        
        // Header
        auto *titleLabel = new QLabel("🚀 BUNKER MINER", this);
        titleLabel->setStyleSheet("font-size: 24px; font-weight: bold; color: #2c3e50; margin: 20px;");
        layout->addWidget(titleLabel);
        
        // Status
        auto *statusLabel = new QLabel("Status: Phase 0.1 - Development Stub", this);
        statusLabel->setStyleSheet("font-size: 14px; color: #27ae60; margin: 10px;");
        layout->addWidget(statusLabel);
        
        // Description
        auto *descLabel = new QLabel(
            "This is the BUNKER MINER desktop client application.\n\n"
            "Phase 0.1: Basic Qt application structure initialized\n"
            "Phase 2.1: Daemon gRPC integration\n"
            "Phase 2.2: Real-time telemetry display\n"
            "Phase 2.4: Complete mining control interface",
            this
        );
        descLabel->setWordWrap(true);
        descLabel->setStyleSheet("margin: 20px; line-height: 1.4;");
        layout->addWidget(descLabel);
        
        // Connect button (stub)
        auto *connectButton = new QPushButton("Connect to Daemon", this);
        connectButton->setStyleSheet(
            "QPushButton {"
            "  background-color: #3498db;"
            "  color: white;"
            "  border: none;"
            "  padding: 10px 20px;"
            "  font-size: 14px;"
            "  border-radius: 5px;"
            "}"
            "QPushButton:hover {"
            "  background-color: #2980b9;"
            "}"
        );
        connect(connectButton, &QPushButton::clicked, this, &BunkerMinerMainWindow::onConnectClicked);
        layout->addWidget(connectButton);
        
        layout->addStretch();
    }
};

int main(int argc, char *argv[])
{
    QApplication app(argc, argv);
    
    // Set application properties
    app.setApplicationName("BUNKER MINER Client");
    app.setApplicationVersion("0.1.0");
    app.setOrganizationName("Bunker Corporation");
    app.setOrganizationDomain("bunkercorpo.com");
    
    std::cout << "BUNKER MINER Client v0.1.0 - Phase 0.1 Stub" << std::endl;
    std::cout << "Qt Version: " << QT_VERSION_STR << std::endl;
    std::cout << "Build: Development stub - full implementation in Phase 2" << std::endl;
    
    BunkerMinerMainWindow window;
    window.show();
    
    return app.exec();
}

#include "main.moc"