import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

// A room or zone: its scenes and its lights.
Kirigami.ScrollablePage {
    id: page

    required property var app
    property string groupId
    readonly property var group: app.group(groupId)
    readonly property var scenes: app.scenesFor(groupId)

    title: group?.name ?? ""

    ListModel { id: lightIds }
    function sync() {
        if (!group) {
            // The room was deleted on the bridge.
            app.pageStack.removePage(page);
            return;
        }
        app.syncIds(lightIds, group.lightIds.slice().sort(
            (a, b) => (app.light(a)?.name ?? "").localeCompare(app.light(b)?.name ?? "")));
    }
    Connections {
        target: page.app
        function onHomeChanged() { page.sync(); }
    }
    Component.onCompleted: sync()

    actions: [
        Kirigami.Action {
            icon.name: "color-picker"
            text: page.app.i18n("Colors")
            enabled: (page.group?.lightIds ?? []).some(id => page.app.light(id)?.supportsColor || page.app.light(id)?.supportsTemperature)
            onTriggered: page.app.openColors(page.groupId)
        }
    ]

    ColumnLayout {
        spacing: Kirigami.Units.largeSpacing

        GroupCard {
            Layout.fillWidth: true
            app: page.app
            groupId: page.groupId
            navigates: false
        }

        Kirigami.ListSectionHeader {
            Layout.fillWidth: true
            visible: page.scenes.length > 0
            text: page.app.i18n("Scenes")
        }
        Flow {
            Layout.fillWidth: true
            spacing: Kirigami.Units.largeSpacing
            Repeater {
                model: page.scenes
                delegate: SceneTile {
                    required property var modelData
                    scene: modelData
                    onClicked: page.app.hue.recallScene(modelData.id)
                }
            }
        }

        Kirigami.ListSectionHeader {
            Layout.fillWidth: true
            text: page.app.i18n("Lights")
        }
        GridLayout {
            Layout.fillWidth: true
            columns: Math.max(1, Math.floor(page.width / (Kirigami.Units.gridUnit * 10)))
            columnSpacing: Kirigami.Units.largeSpacing
            rowSpacing: Kirigami.Units.largeSpacing
            Repeater {
                model: lightIds
                delegate: LightTile {
                    required property string itemId
                    Layout.fillWidth: true
                    app: page.app
                    lightId: itemId
                    onClicked: page.app.openColors(page.groupId, itemId)
                }
            }
        }
    }
}
