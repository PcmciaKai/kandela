#include "app.h"

#include <QApplication>
#include <QIcon>
#include <QLocalServer>
#include <QLocalSocket>
#include <QQmlApplicationEngine>
#include <QQuickStyle>

namespace
{
// A socket path every instance of this user's Kandela can see. Inside a
// Flatpak, /tmp is private to each instance, but $XDG_RUNTIME_DIR/app/<id> is
// shared between them.
QString instanceSocketName()
{
    const QString runtime = qEnvironmentVariable("XDG_RUNTIME_DIR");
    if (runtime.isEmpty()) {
        return QStringLiteral("kandela-%1").arg(qEnvironmentVariable("USER"));
    }
    const QString flatpakId = qEnvironmentVariable("FLATPAK_ID");
    if (!flatpakId.isEmpty()) {
        return QStringLiteral("%1/app/%2/kandela.sock").arg(runtime, flatpakId);
    }
    return runtime + QStringLiteral("/kandela.sock");
}

enum class OtherInstance { None, Activated, Unresponsive };

// If another instance is running, asks it to show its window.
OtherInstance activateRunningInstance()
{
    QLocalSocket socket;
    socket.connectToServer(instanceSocketName());
    if (!socket.waitForConnected(3000)) {
        const auto error = socket.error();
        // Only these mean nobody is listening; a timeout means a busy instance.
        if (error == QLocalSocket::ServerNotFoundError || error == QLocalSocket::ConnectionRefusedError) {
            return OtherInstance::None;
        }
        return OtherInstance::Unresponsive;
    }
    socket.write("show\n");
    socket.waitForBytesWritten(1000);
    return OtherInstance::Activated;
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

    // Starting the app again brings this instance's window up instead.
    QLocalServer server;
    if (single_instance) {
        switch (activateRunningInstance()) {
        case OtherInstance::Activated:
            return 0;
        case OtherInstance::Unresponsive:
            qWarning("Kandela is already running but not responding");
            return 1;
        case OtherInstance::None:
            // Any socket file left is stale (e.g. after a crash).
            QLocalServer::removeServer(instanceSocketName());
            server.setSocketOptions(QLocalServer::UserAccessOption);
            if (!server.listen(instanceSocketName())) {
                qWarning("Kandela could not listen on %s: %s", qPrintable(server.fullServerName()), qPrintable(server.errorString()));
            }
            break;
        }
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

    if (server.isListening()) {
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
