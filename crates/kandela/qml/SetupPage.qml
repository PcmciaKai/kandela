import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

Kirigami.ScrollablePage {
    id: page

    required property var app
    readonly property var hue: app.hue
    readonly property var bridges: JSON.parse(hue.bridgesJson)
    readonly property bool waiting: hue.pairingState === "waiting"
    property string pairingAddress: ""

    title: app.i18n("Set Up")

    function pair(address) {
        pairingAddress = address;
        hue.pair(address);
    }

    actions: [
        Kirigami.Action {
            icon.name: "view-refresh"
            text: app.i18n("Search Again")
            enabled: !hue.discovering && !page.waiting
            onTriggered: hue.discover()
        },
        Kirigami.Action {
            icon.name: "media-playback-start"
            text: app.i18n("Try Demo")
            onTriggered: hue.startDemo()
        }
    ]

    ColumnLayout {
        spacing: Kirigami.Units.largeSpacing

        Kirigami.Icon {
            Layout.alignment: Qt.AlignHCenter
            Layout.topMargin: Kirigami.Units.gridUnit * 2
            implicitWidth: Kirigami.Units.iconSizes.enormous
            implicitHeight: implicitWidth
            source: page.app.archetypes.base + "kandela.svg"
        }

        Kirigami.Heading {
            Layout.fillWidth: true
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.Wrap
            text: page.waiting ? page.app.i18n("Press the link button on your bridge")
                               : page.app.i18n("Connect your Hue Bridge")
        }

        QQC2.Label {
            Layout.fillWidth: true
            Layout.maximumWidth: Kirigami.Units.gridUnit * 26
            Layout.alignment: Qt.AlignHCenter
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.Wrap
            color: Kirigami.Theme.disabledTextColor
            text: page.waiting
                ? page.app.i18n("It's the large round button on top of the bridge at %1. This window will continue on its own once you've pressed it.", page.pairingAddress)
                : page.app.i18n("Your computer talks to the bridge directly over your home network. No Hue account is needed.")
        }

        QQC2.BusyIndicator {
            Layout.alignment: Qt.AlignHCenter
            visible: page.waiting
            running: visible
        }

        QQC2.Button {
            Layout.alignment: Qt.AlignHCenter
            visible: page.waiting
            text: page.app.i18n("Cancel")
            onClicked: page.hue.cancelPairing()
        }

        Kirigami.InlineMessage {
            Layout.fillWidth: true
            Layout.maximumWidth: Kirigami.Units.gridUnit * 30
            Layout.alignment: Qt.AlignHCenter
            type: Kirigami.MessageType.Error
            visible: page.hue.pairingState === "failed"
            text: page.hue.pairingMessage
        }

        Kirigami.InlineMessage {
            Layout.fillWidth: true
            Layout.maximumWidth: Kirigami.Units.gridUnit * 30
            Layout.alignment: Qt.AlignHCenter
            type: Kirigami.MessageType.Warning
            visible: page.hue.message.length > 0 && !page.waiting
            text: page.hue.message
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.maximumWidth: Kirigami.Units.gridUnit * 30
            Layout.alignment: Qt.AlignHCenter
            visible: !page.waiting
            spacing: 0

            Kirigami.ListSectionHeader {
                Layout.fillWidth: true
                text: page.app.i18n("Bridges on this network")
            }

            RowLayout {
                visible: page.hue.discovering
                Layout.margins: Kirigami.Units.largeSpacing
                spacing: Kirigami.Units.largeSpacing
                QQC2.BusyIndicator {
                    running: visible
                    implicitWidth: Kirigami.Units.iconSizes.medium
                    implicitHeight: implicitWidth
                }
                QQC2.Label { text: page.app.i18n("Searching…") }
            }

            QQC2.Label {
                Layout.margins: Kirigami.Units.largeSpacing
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                visible: !page.hue.discovering && page.bridges.length === 0
                color: Kirigami.Theme.disabledTextColor
                text: page.app.i18n("No bridge found. Make sure it's powered on and connected to your router, or enter its address below.")
            }

            Repeater {
                model: page.bridges
                delegate: QQC2.ItemDelegate {
                    required property var modelData
                    Layout.fillWidth: true
                    icon.name: "network-wired"
                    text: modelData.name || page.app.i18n("Hue Bridge")
                    onClicked: page.pair(modelData.address)

                    contentItem: RowLayout {
                        spacing: Kirigami.Units.largeSpacing
                        Kirigami.Icon {
                            source: "network-wired"
                            implicitWidth: Kirigami.Units.iconSizes.medium
                            implicitHeight: implicitWidth
                        }
                        ColumnLayout {
                            spacing: 0
                            Layout.fillWidth: true
                            QQC2.Label { text: modelData.name || page.app.i18n("Hue Bridge") }
                            QQC2.Label {
                                text: modelData.address + (modelData.model ? " · " + modelData.model : "")
                                color: Kirigami.Theme.disabledTextColor
                                font: Kirigami.Theme.smallFont
                            }
                        }
                        QQC2.Label { text: page.app.i18n("Connect") ; color: Kirigami.Theme.linkColor }
                    }
                }
            }

            Kirigami.ListSectionHeader {
                Layout.fillWidth: true
                Layout.topMargin: Kirigami.Units.largeSpacing
                text: page.app.i18n("Enter address manually")
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.margins: Kirigami.Units.smallSpacing
                QQC2.TextField {
                    id: addressField
                    Layout.fillWidth: true
                    placeholderText: page.app.i18n("e.g. 192.168.1.20")
                    inputMethodHints: Qt.ImhNoAutoUppercase | Qt.ImhUrlCharactersOnly
                    onAccepted: if (text.trim().length > 0) page.pair(text)
                }
                QQC2.Button {
                    text: page.app.i18n("Connect")
                    enabled: addressField.text.trim().length > 0
                    onClicked: page.pair(addressField.text)
                }
            }
        }
    }
}
