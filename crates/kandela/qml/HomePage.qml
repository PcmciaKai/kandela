import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

Kirigami.ScrollablePage {
    id: page

    required property var app
    readonly property var hue: app.hue

    // Screenshots of the demo should look like a real home.
    readonly property bool showDemoHints: hue.status === "demo" && app.screenshotPath.length === 0
                                          && app.screenshotRoute.length === 0

    title: showDemoHints ? app.i18n("Demo Home") : app.i18n("Home")

    ListModel { id: roomIds }
    ListModel { id: zoneIds }

    function sync() {
        app.syncIds(roomIds, app.home.rooms.map(r => r.id));
        app.syncIds(zoneIds, app.home.zones.map(z => z.id));
    }
    Connections {
        target: page.app
        function onHomeChanged() { page.sync(); }
    }
    Component.onCompleted: sync()

    readonly property int columns: Math.max(1, Math.floor((width - Kirigami.Units.gridUnit * 2) / (Kirigami.Units.gridUnit * 17)))

    actions: [
        Kirigami.Action {
            icon.name: page.app.anyOn ? "system-shutdown" : "brightness-high"
            text: page.app.anyOn ? page.app.i18n("All Off") : page.app.i18n("All On")
            enabled: page.app.home.rooms.length > 0
            onTriggered: page.hue.setAllOn(!page.app.anyOn)
        },
        Kirigami.Action {
            icon.name: "network-connect"
            text: page.app.i18n("Connect a Bridge…")
            visible: page.showDemoHints
            onTriggered: page.hue.forgetBridge()
        },
        Kirigami.Action {
            icon.name: "window-minimize-symbolic"
            text: page.app.i18n("Keep Running in System Tray When Closed")
            checkable: true
            checked: page.app.settings.closeToTray
            visible: page.app.tray.available
            displayHint: Kirigami.DisplayHint.AlwaysHide
            onTriggered: page.app.settings.closeToTray = checked
        },
        Kirigami.Action {
            icon.name: "application-exit"
            text: page.app.i18n("Quit")
            displayHint: Kirigami.DisplayHint.AlwaysHide
            onTriggered: Qt.quit()
        },
        Kirigami.Action {
            icon.name: "network-disconnect"
            text: page.app.i18n("Disconnect Bridge…")
            visible: page.hue.status !== "demo"
            displayHint: Kirigami.DisplayHint.AlwaysHide
            onTriggered: forgetDialog.open()
        }
    ]

    Kirigami.PromptDialog {
        id: forgetDialog
        title: page.app.i18n("Disconnect Bridge?")
        subtitle: page.app.i18n("This computer will forget %1. You'll need to press the link button again to reconnect.", page.hue.bridgeName)
        standardButtons: Kirigami.Dialog.Cancel
        customFooterActions: Kirigami.Action {
            text: page.app.i18n("Disconnect")
            icon.name: "network-disconnect"
            onTriggered: {
                page.hue.forgetBridge();
                forgetDialog.close();
            }
        }
    }

    header: ColumnLayout {
        spacing: 0
        Kirigami.InlineMessage {
            Layout.fillWidth: true
            position: Kirigami.InlineMessage.Position.Header
            visible: page.showDemoHints
            type: Kirigami.MessageType.Information
            text: page.app.i18n("This is a demo home. Changes only happen in this window.")
        }
        Kirigami.InlineMessage {
            Layout.fillWidth: true
            position: Kirigami.InlineMessage.Position.Header
            visible: page.hue.status === "offline"
            type: Kirigami.MessageType.Warning
            text: page.app.i18n("Can't reach %1. Retrying… (%2)", page.hue.bridgeName, page.hue.message)
            actions: Kirigami.Action {
                text: page.app.i18n("Retry Now")
                icon.name: "view-refresh"
                onTriggered: page.hue.reconnect()
            }
        }
        Kirigami.InlineMessage {
            Layout.fillWidth: true
            position: Kirigami.InlineMessage.Position.Header
            visible: page.hue.status === "untrusted"
            type: Kirigami.MessageType.Error
            text: page.app.i18n("The device at %1 doesn't match the bridge you paired with, so this app stopped talking to it. This happens when the bridge was reset or replaced, or its address now belongs to another device — but it could also mean something on your network is pretending to be your bridge.", page.hue.message)
            actions: Kirigami.Action {
                text: page.app.i18n("Pair Again…")
                icon.name: "network-connect"
                onTriggered: page.hue.forgetBridge()
            }
        }
        Kirigami.InlineMessage {
            Layout.fillWidth: true
            position: Kirigami.InlineMessage.Position.Header
            // Not showCloseButton: it hides the message by breaking this binding.
            visible: page.hue.status === "connected" && page.hue.errorMessage.length > 0
            type: Kirigami.MessageType.Error
            text: page.hue.errorMessage
            actions: Kirigami.Action {
                text: page.app.i18n("Dismiss")
                icon.name: "dialog-close"
                onTriggered: page.hue.errorMessage = ""
            }
        }
    }

    ColumnLayout {
        spacing: Kirigami.Units.largeSpacing

        Kirigami.LoadingPlaceholder {
            Layout.alignment: Qt.AlignCenter
            Layout.topMargin: Kirigami.Units.gridUnit * 4
            visible: page.hue.status === "connecting"
            text: page.app.i18n("Connecting to %1…", page.hue.bridgeName)
        }

        Kirigami.PlaceholderMessage {
            Layout.fillWidth: true
            Layout.topMargin: Kirigami.Units.gridUnit * 4
            visible: (page.hue.status === "connected" || page.hue.status === "demo") && roomIds.count === 0 && zoneIds.count === 0
            icon.name: "lighttable"
            text: page.app.i18n("No rooms yet")
            explanation: page.app.i18n("Rooms you create in the Hue app on your phone appear here.")
        }

        Kirigami.ListSectionHeader {
            Layout.fillWidth: true
            visible: roomIds.count > 0
            text: page.app.i18n("Rooms")
        }
        GridLayout {
            Layout.fillWidth: true
            columns: page.columns
            columnSpacing: Kirigami.Units.largeSpacing
            rowSpacing: Kirigami.Units.largeSpacing
            Repeater {
                model: roomIds
                delegate: GroupCard {
                    required property string itemId
                    Layout.fillWidth: true
                    Layout.preferredWidth: Kirigami.Units.gridUnit * 16
                    app: page.app
                    groupId: itemId
                }
            }
        }

        Kirigami.ListSectionHeader {
            Layout.fillWidth: true
            Layout.topMargin: Kirigami.Units.largeSpacing
            visible: zoneIds.count > 0
            text: page.app.i18n("Zones")
        }
        GridLayout {
            Layout.fillWidth: true
            columns: page.columns
            columnSpacing: Kirigami.Units.largeSpacing
            rowSpacing: Kirigami.Units.largeSpacing
            Repeater {
                model: zoneIds
                delegate: GroupCard {
                    required property string itemId
                    Layout.fillWidth: true
                    Layout.preferredWidth: Kirigami.Units.gridUnit * 16
                    app: page.app
                    groupId: itemId
                }
            }
        }
    }
}
