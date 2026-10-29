import QtQuick
import Qt.labs.platform as Platform

// The system tray icon: a lit bulb while any light is on. Clicking it shows or
// hides the window; its menu switches rooms and zones.
Platform.SystemTrayIcon {
    id: tray

    required property var app

    visible: true
    icon.source: app.archetypes.base + (app.anyOn ? "tray-on.svg" : "tray-off.svg")
    tooltip: app.i18n("Kandela")

    onActivated: reason => {
        if (reason === Platform.SystemTrayIcon.Trigger) app.toggleWindow();
    }

    // SystemTrayIcon takes no child objects, so helpers live in properties.
    // Menu items can't be moved, so the menu is rebuilt whenever the rooms
    // and zones or their order change (not on every state update).
    property var groupIds: []
    function sync() {
        const ids = app.home.rooms.concat(app.home.zones).map(g => g.id);
        if (JSON.stringify(ids) !== JSON.stringify(groupIds)) groupIds = ids;
    }
    readonly property Connections homeWatcher: Connections {
        target: tray.app
        function onHomeChanged() { tray.sync(); }
    }
    Component.onCompleted: sync()

    // Rooms and zones go between the first separator and "All Off".
    readonly property int groupsStart: 2

    menu: Platform.Menu {
        id: menu

        Platform.MenuItem {
            text: tray.app.visible && tray.app.visibility !== Window.Minimized
                ? tray.app.i18n("Hide Kandela") : tray.app.i18n("Show Kandela")
            onTriggered: tray.app.toggleWindow()
        }
        Platform.MenuSeparator {}
        Platform.MenuItem {
            text: tray.app.anyOn ? tray.app.i18n("All Off") : tray.app.i18n("All On")
            visible: tray.app.home.rooms.length > 0
            onTriggered: tray.app.hue.setAllOn(!tray.app.anyOn)
        }
        Platform.MenuSeparator {}
        Platform.MenuItem {
            text: tray.app.i18n("Quit")
            onTriggered: Qt.quit()
        }
    }

    readonly property Instantiator groupItems: Instantiator {
        model: tray.groupIds
        delegate: Platform.MenuItem {
            required property string modelData
            readonly property var entry: tray.app.group(modelData)
            text: entry?.name ?? ""
            checkable: true
            checked: entry?.on ?? false
            // `checked` has already flipped when this runs.
            onTriggered: tray.app.hue.setGroupOn(entry.groupedLightId, checked)
        }
        onObjectAdded: (index, item) => menu.insertItem(tray.groupsStart + index, item)
        onObjectRemoved: (index, item) => menu.removeItem(item)
    }
}
