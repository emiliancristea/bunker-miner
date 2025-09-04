#pragma once

#include <QMainWindow>
#include <QVBoxLayout>
#include <QLabel>
#include <QPushButton>
#include <QWidget>

/**
 * MainWindow class for BUNKER MINER Client
 * 
 * Phase 0.1: Basic stub implementation
 * Phase 2.1: Full implementation with daemon integration
 */
class MainWindow : public QMainWindow {
    Q_OBJECT

public:
    explicit MainWindow(QWidget *parent = nullptr);
    ~MainWindow() override = default;

private slots:
    void onConnectDaemon();
    void onStartMining();
    void onStopMining();

private:
    void setupUI();
    void updateStatus(const QString &status);

    // UI Components (stubs for Phase 2 implementation)
    QWidget *m_centralWidget;
    QVBoxLayout *m_mainLayout;
    QLabel *m_titleLabel;
    QLabel *m_statusLabel;
    QPushButton *m_connectButton;
    QPushButton *m_startButton;
    QPushButton *m_stopButton;
};