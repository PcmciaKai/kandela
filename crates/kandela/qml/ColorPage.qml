import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

// Picking colors and white tones for the lights of one room or zone.
Kirigami.ScrollablePage {
    id: page

    required property var app
    property string groupId
    // Empty means "all lights": clicking the wheel colors every light.
    property string selectedLightId
    property bool whiteMode: false

    readonly property var group: app.group(groupId)
    readonly property var lights: (group?.lightIds ?? []).map(id => app.light(id)).filter(l => l)
        .sort((a, b) => a.name.localeCompare(b.name))
    readonly property var selected: app.light(selectedLightId)
    readonly property var colorLights: lights.filter(l => l.supportsColor)
    readonly property var whiteLights: lights.filter(l => l.supportsTemperature)

    title: app.i18n("%1 Colors", group?.name ?? "")

    Component.onCompleted: {
        const l = selected;
        whiteMode = l ? (!l.supportsColor || l.mode === "temperature")
                      : colorLights.length === 0;
    }

    function forTargets(fn) {
        const targets = selectedLightId ? [selectedLightId]
                      : (whiteMode ? whiteLights : colorLights).map(l => l.id);
        targets.forEach(fn);
    }

    ColumnLayout {
        spacing: Kirigami.Units.largeSpacing

        RowLayout {
            Layout.alignment: Qt.AlignHCenter
            spacing: 0
            QQC2.ButtonGroup { id: modeGroup }
            QQC2.Button {
                text: page.app.i18n("Color")
                icon.name: "color-management"
                checkable: true
                checked: !page.whiteMode
                enabled: page.colorLights.length > 0
                QQC2.ButtonGroup.group: modeGroup
                onClicked: page.whiteMode = false
            }
            QQC2.Button {
                text: page.app.i18n("White")
                icon.name: "brightness-high"
                checkable: true
                checked: page.whiteMode
                enabled: page.whiteLights.length > 0
                QQC2.ButtonGroup.group: modeGroup
                onClicked: page.whiteMode = true
            }
        }

        ColorWheel {
            Layout.alignment: Qt.AlignHCenter
            Layout.preferredWidth: Math.min(page.width - Kirigami.Units.gridUnit * 2, Kirigami.Units.gridUnit * 22)
            Layout.preferredHeight: Layout.preferredWidth
            visible: !page.whiteMode
            selectedId: page.selectedLightId
            markers: page.colorLights
            onMarkerSelected: id => page.selectedLightId = id
            onColorPicked: (id, h, s) => page.app.hue.setLightHueSaturation(id, h, s)
            onBackgroundPicked: (h, s) => page.forTargets(id => page.app.hue.setLightHueSaturation(id, h, s))
        }

        TemperatureBar {
            Layout.fillWidth: true
            Layout.maximumWidth: Kirigami.Units.gridUnit * 26
            Layout.alignment: Qt.AlignHCenter
            Layout.topMargin: Kirigami.Units.gridUnit * 2
            Layout.bottomMargin: Kirigami.Units.gridUnit * 2
            visible: page.whiteMode
            app: page.app
            selectedId: page.selectedLightId
            markers: page.whiteLights
            onMarkerSelected: id => page.selectedLightId = id
            onMirekPicked: (id, m) => page.app.hue.setLightMirek(id, m)
            onBackgroundPicked: m => page.forTargets(id => page.app.hue.setLightMirek(id, m))
        }

        QQC2.Label {
            Layout.fillWidth: true
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.Wrap
            color: Kirigami.Theme.disabledTextColor
            text: page.selectedLightId
                ? page.app.i18n("Drag a marker, or click anywhere to move %1 there.", page.selected?.name ?? "")
                : page.app.i18n("Click anywhere to set all lights, or drag a single marker.")
        }

        Kirigami.ListSectionHeader {
            Layout.fillWidth: true
            text: page.app.i18n("Lights")
        }

        Flow {
            Layout.fillWidth: true
            spacing: Kirigami.Units.smallSpacing

            LightChip {
                text: page.app.i18n("All Lights")
                showDot: false
                checked: page.selectedLightId === ""
                onClicked: page.selectedLightId = ""
            }
            Repeater {
                model: page.lights
                delegate: LightChip {
                    required property var modelData
                    text: modelData.name
                    dotColor: modelData.on ? modelData.color : "transparent"
                    checked: page.selectedLightId === modelData.id
                    onClicked: page.selectedLightId = checked ? "" : modelData.id
                }
            }
        }

        Kirigami.FormLayout {
            Layout.fillWidth: true

            QQC2.Switch {
                Kirigami.FormData.label: page.app.i18n("Power:")
                checked: page.selected ? page.selected.on : (page.group?.on ?? false)
                onToggled: page.selected ? page.app.hue.setLightOn(page.selectedLightId, checked)
                                         : page.app.hue.setGroupOn(page.group.groupedLightId, checked)
            }

            RowLayout {
                Kirigami.FormData.label: page.app.i18n("Brightness:")
                QQC2.Slider {
                    id: brightness
                    Layout.preferredWidth: Kirigami.Units.gridUnit * 14
                    from: 1
                    to: 100
                    onMoved: page.selected ? page.app.hue.setLightBrightness(page.selectedLightId, value)
                                           : page.app.hue.setGroupBrightness(page.group.groupedLightId, value)
                    Binding on value {
                        value: page.selected ? page.selected.brightness : (page.group?.brightness ?? 0)
                        when: !brightness.pressed
                        restoreMode: Binding.RestoreNone
                    }
                }
                QQC2.Label {
                    text: page.app.i18n("%1%", Math.round(brightness.value))
                    Layout.minimumWidth: Kirigami.Units.gridUnit * 2.5
                }
            }
        }
    }
}
