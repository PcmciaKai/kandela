import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

// One light: its color, name, brightness and an on/off switch.
QQC2.AbstractButton {
    id: tile

    required property var app
    required property string lightId
    readonly property var light: app.light(lightId)
    readonly property bool on: light?.on ?? false
    readonly property bool reachable: light?.reachable ?? true

    hoverEnabled: true
    padding: Kirigami.Units.largeSpacing
    implicitWidth: Kirigami.Units.gridUnit * 9
    implicitHeight: implicitContentHeight + topPadding + bottomPadding
    Accessible.name: light?.name ?? ""

    background: Kirigami.ShadowedRectangle {
        Kirigami.Theme.colorSet: Kirigami.Theme.View
        Kirigami.Theme.inherit: false
        radius: Kirigami.Units.cornerRadius * 2
        color: Kirigami.Theme.backgroundColor
        border.width: 1
        border.color: tile.visualFocus ? Kirigami.Theme.focusColor
                    : tile.hovered ? Kirigami.Theme.hoverColor
                    : Kirigami.ColorUtils.linearInterpolation(Kirigami.Theme.backgroundColor, Kirigami.Theme.textColor, 0.15)
        shadow.size: tile.hovered ? Kirigami.Units.largeSpacing : Kirigami.Units.smallSpacing
        shadow.color: Qt.rgba(0, 0, 0, 0.12)
        shadow.yOffset: 1

        Rectangle {
            anchors.fill: parent
            anchors.margins: 1
            radius: parent.radius - 1
            opacity: tile.on && tile.reachable ? 0.38 : 0
            Behavior on opacity { NumberAnimation { duration: Kirigami.Units.longDuration } }
            gradient: Gradient {
                GradientStop { position: 0.0; color: tile.light?.color ?? "transparent" }
                GradientStop { position: 1.0; color: Qt.alpha(tile.light?.color ?? "#000000", 0.25) }
            }
        }
    }

    contentItem: ColumnLayout {
        spacing: Kirigami.Units.smallSpacing

        RowLayout {
            Image {
                sourceSize.width: Kirigami.Units.iconSizes.smallMedium
                sourceSize.height: Kirigami.Units.iconSizes.smallMedium
                source: tile.app.archetypes.light(tile.light?.archetype ?? "",
                    tile.on ? Kirigami.Theme.textColor : Kirigami.Theme.disabledTextColor)
            }
            Item { Layout.fillWidth: true }
            QQC2.Switch {
                checked: tile.on
                enabled: tile.reachable
                onToggled: tile.app.hue.setLightOn(tile.lightId, checked)
                Accessible.name: tile.app.i18n("Turn %1 on or off", tile.light?.name ?? "")
            }
        }

        QQC2.Label {
            Layout.fillWidth: true
            text: tile.light?.name ?? ""
            elide: Text.ElideRight
            font.weight: Font.DemiBold
        }
        QQC2.Label {
            Layout.fillWidth: true
            font: Kirigami.Theme.smallFont
            opacity: 0.75
            elide: Text.ElideRight
            text: !tile.reachable ? tile.app.i18n("Unreachable")
                : tile.on ? tile.app.i18n("%1%", Math.round(tile.light.brightness))
                : tile.app.i18n("Off")
        }
    }
}
