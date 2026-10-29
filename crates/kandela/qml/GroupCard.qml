import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

// A room or zone: name, state, on/off switch and brightness, tinted with the
// colors its lights are showing.
QQC2.AbstractButton {
    id: card

    required property var app
    required property string groupId
    // Opens the room's page when clicked.
    property bool navigates: true
    readonly property var group: app.group(groupId)
    readonly property bool on: group?.on ?? false
    readonly property var colors: group?.colors ?? []
    readonly property bool darkTheme: Kirigami.ColorUtils.brightnessForColor(Kirigami.Theme.backgroundColor) === Kirigami.ColorUtils.Dark

    function statusText() {
        if (!group) return "";
        const total = group.lightIds.length;
        if (total === 0) return app.i18n("No lights");
        if (group.lightsOn === 0) return app.i18n("Off");
        if (group.lightsOn === total) return total === 1 ? app.i18n("On") : app.i18n("All lights on");
        return app.i18n("%1 of %2 lights on", group.lightsOn, total);
    }

    hoverEnabled: navigates
    focusPolicy: navigates ? Qt.StrongFocus : Qt.NoFocus
    padding: Kirigami.Units.largeSpacing
    implicitHeight: implicitContentHeight + topPadding + bottomPadding
    Accessible.name: group?.name ?? ""
    Accessible.description: statusText()

    background: Kirigami.ShadowedRectangle {
        radius: Kirigami.Units.cornerRadius * 2
        color: Kirigami.Theme.backgroundColor
        border.width: 1
        border.color: card.visualFocus ? Kirigami.Theme.focusColor
                    : card.hovered ? Kirigami.Theme.hoverColor
                    : Kirigami.ColorUtils.linearInterpolation(Kirigami.Theme.backgroundColor, Kirigami.Theme.textColor, 0.15)
        shadow.size: card.hovered ? Kirigami.Units.largeSpacing : Kirigami.Units.smallSpacing
        shadow.color: Qt.rgba(0, 0, 0, 0.12)
        shadow.yOffset: 1

        Kirigami.Theme.colorSet: Kirigami.Theme.View
        Kirigami.Theme.inherit: false

        // The lights' colors, as a soft wash behind the content.
        Rectangle {
            anchors.fill: parent
            anchors.margins: 1
            radius: parent.radius - 1
            opacity: card.on && card.colors.length > 0 ? (card.darkTheme ? 0.42 : 0.32) : 0
            Behavior on opacity { NumberAnimation { duration: Kirigami.Units.longDuration } }
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0.0; color: card.colors[0] ?? "transparent" }
                GradientStop { position: 0.55; color: card.colors[1] ?? card.colors[0] ?? "transparent" }
                GradientStop { position: 1.0; color: Qt.alpha(card.colors[2] ?? card.colors[1] ?? card.colors[0] ?? "#000000", 0.35) }
            }
        }
    }

    contentItem: ColumnLayout {
        spacing: Kirigami.Units.smallSpacing

        RowLayout {
            spacing: Kirigami.Units.largeSpacing

            Rectangle {
                implicitWidth: Kirigami.Units.iconSizes.medium + Kirigami.Units.largeSpacing
                implicitHeight: implicitWidth
                radius: width / 2
                color: card.on ? Qt.alpha(card.colors[0] ?? Kirigami.Theme.highlightColor, 0.35)
                               : Qt.alpha(Kirigami.Theme.textColor, 0.08)
                Image {
                    anchors.centerIn: parent
                    sourceSize.width: Kirigami.Units.iconSizes.smallMedium
                    sourceSize.height: Kirigami.Units.iconSizes.smallMedium
                    source: card.app.archetypes.room(card.group?.archetype ?? "", Kirigami.Theme.textColor)
                }
            }

            ColumnLayout {
                spacing: 0
                Layout.fillWidth: true
                Kirigami.Heading {
                    Layout.fillWidth: true
                    level: 4
                    text: card.group?.name ?? ""
                    elide: Text.ElideRight
                }
                QQC2.Label {
                    Layout.fillWidth: true
                    text: card.statusText()
                    elide: Text.ElideRight
                    font: Kirigami.Theme.smallFont
                    opacity: 0.75
                }
            }

            QQC2.Switch {
                checked: card.on
                onToggled: card.app.hue.setGroupOn(card.group.groupedLightId, checked)
                Accessible.name: card.app.i18n("Turn %1 on or off", card.group?.name ?? "")
            }
        }

        QQC2.Slider {
            id: slider
            Layout.fillWidth: true
            from: 1
            to: 100
            enabled: (card.group?.lightIds.length ?? 0) > 0
            opacity: card.on ? 1 : 0.6
            onMoved: card.app.hue.setGroupBrightness(card.group.groupedLightId, value)
            Binding on value {
                value: card.group?.brightness ?? 0
                when: !slider.pressed
                restoreMode: Binding.RestoreNone
            }
            Accessible.name: card.app.i18n("Brightness")
        }
    }

    onClicked: if (navigates) app.openGroup(groupId)
}
