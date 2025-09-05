#include <QApplication>
#include <iostream>
#include "MainWindow.h"

/**
 * BUNKER MINER Client - Phase 2.1 Implementation
 * 
 * Main entry point for the BUNKER MINER desktop client application.
 * Features:
 * - Complete GUI application shell with navigation
 * - gRPC daemon integration for system information
 * - Real-time connection status and error handling
 * - Cross-platform Qt-based user interface
 */

int main(int argc, char *argv[])
{
    QApplication app(argc, argv);
    
    // Set application properties
    app.setApplicationName("BUNKER MINER Client");
    app.setApplicationVersion("2.1.0");
    app.setOrganizationName("Bunker Corporation");
    app.setOrganizationDomain("bunkercorp.com");
    
    // Application metadata
    std::cout << "BUNKER MINER Client v2.1.0 - Phase 2.1" << std::endl;
    std::cout << "Qt Version: " << QT_VERSION_STR << std::endl;
    std::cout << "Build: C++/Qt GUI Shell with gRPC Daemon Integration" << std::endl;
    std::cout << "Features: Navigation UI, System Info Display, Connection Management" << std::endl;
    
    // Create and show main window
    MainWindow window;
    window.show();
    
    std::cout << "Application started successfully. Attempting daemon connection..." << std::endl;
    
    return app.exec();
}