#include "app.h"

#include <QApplication>
#include <QIcon>
#include <QLocalServer>
#include <QLocalSocket>
#include <QQmlApplicationEngine>
#include <QQuickStyle>

namespace
{
QString instanceSocketName()
{
    return QStringLiteral("kandela-%1").arg(qEnvironmentVariable("USER"));
}

// If another instance is running, asks it to show its window and returns true.
bool activateRunningInstance()
{
    QLocalSocket socket;
    socket.connectToServer(instanceSocketName());
    if (!socket.waitForConnected(500)) {
        return false;
    }
    socket.write("show\n");
    socket.waitForBytesWritten(500);
    return true;
}
}

std::int32_t run_app(bool start_hidden, bool single_instance)
{
    static int argc = 1;
    static char arg0[] = "kandela";
    static char *argv[] = {arg0, nullptr};

    QApplication app(argc, argv);
    QApplication::setApplicationName(QStringLiteral("kandela"));
    // Settings land in ~/.config/kandela/kandela.conf.
    QApplication::setOrganizationName(QStringLiteral("kandela"));
    // The window may be hidden in the tray; Main.qml decides when to quit.
    QApplication::setQuitOnLastWindowClosed(false);

    if (single_instance && activateRunningInstance()) {
        return 0;
    }
    QApplication::setApplicationDisplayName(QStringLiteral("Kandela"));
    QApplication::setDesktopFileName(QStringLiteral("io.github.pcmciakai.kandela"));
    QApplication::setWindowIcon(QIcon(QStringLiteral(":/qt/qml/io/github/pcmciakai/kandela/qml/icons/kandela.svg")));

    // Match Plasma's look (Breeze controls, the user's color scheme). Outside
    // Plasma, or without the KDE style installed, Qt falls back to Fusion.
    if (qEnvironmentVariableIsEmpty("QT_QUICK_CONTROLS_STYLE")) {
        QQuickStyle::setStyle(QStringLiteral("org.kde.desktop"));
    }
    QQuickStyle::setFallbackStyle(QStringLiteral("Fusion"));
    if (QIcon::themeName().isEmpty()) {
        QIcon::setThemeName(QStringLiteral("breeze"));
    }

    QQmlApplicationEngine engine;
    QObject::connect(
        &engine, &QQmlApplicationEngine::objectCreationFailed, &app,
        [] { QCoreApplication::exit(1); }, Qt::QueuedConnection);
    // Development aid: open a page, save a screenshot and quit.
    //   KANDELA_SCREENSHOT=out.png KANDELA_ROUTE="colors:Living room" kandela --demo
    engine.setInitialProperties({
        {QStringLiteral("screenshotPath"), qEnvironmentVariable("KANDELA_SCREENSHOT")},
        {QStringLiteral("screenshotRoute"), qEnvironmentVariable("KANDELA_ROUTE")},
        {QStringLiteral("startHidden"), start_hidden},
    });
    engine.loadFromModule("io.github.pcmciakai.kandela", "Main");

    // Starting the app again brings this instance's window up instead.
    QLocalServer server;
    if (single_instance) {
        QLocalServer::removeServer(instanceSocketName());
        server.setSocketOptions(QLocalServer::UserAccessOption);
        server.listen(instanceSocketName());
        QObject::connect(&server, &QLocalServer::newConnection, &server, [&server, &engine] {
            while (QLocalSocket *client = server.nextPendingConnection()) {
                QObject::connect(client, &QLocalSocket::disconnected, client, &QObject::deleteLater);
                if (!engine.rootObjects().isEmpty()) {
                    QMetaObject::invokeMethod(engine.rootObjects().constFirst(), "showWindow");
                }
            }
        });
    }
    return app.exec();
}
