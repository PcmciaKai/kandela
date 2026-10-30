import QtCore
import QtQuick
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import io.github.pcmciakai.kandela

Kirigami.ApplicationWindow {
    id: root

    title: i18n("Kandela")
    width: Kirigami.Units.gridUnit * 52
    // Fits Flathub's screenshot guideline (at most 1000x700 with the title bar).
    height: Kirigami.Units.gridUnit * 36
    minimumWidth: Kirigami.Units.gridUnit * 20
    minimumHeight: Kirigami.Units.gridUnit * 24

    // With --hidden the app starts in the tray (if there is one).
    property bool startHidden: false
    visible: !startHidden || !tray.available

    pageStack.defaultColumnWidth: Kirigami.Units.gridUnit * 24
    pageStack.globalToolBar.style: Kirigami.ApplicationHeaderStyle.ToolBar
    pageStack.globalToolBar.showNavigationButtons: Kirigami.ApplicationHeaderStyle.ShowBackButton

    // Simple i18n stand-in until the app is hooked up to KI18n.
    function i18n(text, ...args) {
        let out = text;
        args.forEach((a, i) => out = out.replace("%" + (i + 1), a));
        return out;
    }

    readonly property HueController hue: HueController {
        id: hue
    }

    // The home as last reported by the service, plus lookup tables.
    property var home: ({ rooms: [], zones: [], lights: [], scenes: [] })
    property var lightMap: ({})
    property var groupMap: ({})

    readonly property var archetypes: Archetypes {}

    readonly property Settings settings: Settings {
        property bool closeToTray: true
        property bool trayHintShown: false
    }

    readonly property Tray tray: Tray {
        id: tray
        app: root
    }

    function showWindow() {
        if (visibility === Window.Minimized) showNormal();
        else show();
        raise();
        requestActivate();
    }

    function toggleWindow() {
        if (visible && visibility !== Window.Minimized) hide();
        else showWindow();
    }

    // Closing hides the window to the tray, unless turned off or there's no tray.
    onClosing: close => {
        if (!settings.closeToTray || !tray.available) {
            Qt.quit();
            return;
        }
        close.accepted = false;
        hide();
        if (!settings.trayHintShown) {
            settings.trayHintShown = true;
            tray.showMessage(i18n("Kandela is still running"),
                i18n("Click the bulb in the system tray to open it again, or right-click it to switch rooms and quit."));
        }
    }

    Shortcut {
        sequences: [StandardKey.Quit]
        onActivated: Qt.quit()
    }

    function light(id) { return lightMap[id] ?? null; }
    function group(id) { return groupMap[id] ?? null; }
    function scenesFor(groupId) { return home.scenes.filter(s => s.groupId === groupId); }
    readonly property bool anyOn: home.rooms.some(r => r.on)

    function applyHome() {
        const h = JSON.parse(hue.homeJson);
        const next = {
            rooms: h.rooms ?? [], zones: h.zones ?? [],
            lights: h.lights ?? [], scenes: h.scenes ?? []
        };
        const lights = {};
        next.lights.forEach(l => lights[l.id] = l);
        const groups = {};
        next.rooms.concat(next.zones).forEach(g => groups[g.id] = g);
        lightMap = lights;
        groupMap = groups;
        home = next;
    }

    // Keeps a ListModel's `itemId` rows equal to `ids` with as few changes as
    // possible, so delegates (and any slider being dragged) survive updates.
    function syncIds(model, ids) {
        for (let i = model.count - 1; i >= 0; --i) {
            if (!ids.includes(model.get(i).itemId)) model.remove(i);
        }
        ids.forEach((id, i) => {
            let at = -1;
            for (let j = i; j < model.count; ++j) {
                if (model.get(j).itemId === id) { at = j; break; }
            }
            if (at === -1) model.insert(i, { itemId: id });
            else if (at !== i) model.move(at, i, 1);
        });
    }

    function showStatusPage() {
        if (hue.status === "setup") {
            if (!(pageStack.items[0] instanceof SetupPage)) {
                pageStack.clear();
                pageStack.push(setupPage);
            }
        } else if (!(pageStack.items[0] instanceof HomePage)) {
            pageStack.clear();
            pageStack.push(homePage);
        }
    }

    function openGroup(groupId) {
        while (pageStack.depth > 1) pageStack.pop();
        pageStack.push(groupPage, { groupId: groupId });
    }

    function openColors(groupId, lightId) {
        while (pageStack.depth > 2) pageStack.pop();
        pageStack.push(colorPage, { groupId: groupId, selectedLightId: lightId ?? "" });
    }

    Connections {
        target: hue
        function onHomeJsonChanged() { root.applyHome(); }
        function onStatusChanged() { root.showStatusPage(); }
    }

    Component.onCompleted: {
        applyHome();
        // Wait until the page stack has finished setting itself up.
        Qt.callLater(showStatusPage);
    }

    // Set from the environment by app.cpp; see there.
    property string screenshotPath
    property string screenshotRoute
    Timer {
        running: root.screenshotPath.length > 0 || root.screenshotRoute.length > 0
        interval: 1500
        onTriggered: {
            const [kind, name] = root.screenshotRoute.split(":");
            const g = root.home.rooms.concat(root.home.zones).find(g => g.name === name);
            if (kind === "group" && g) root.openGroup(g.id);
            if ((kind === "colors" || kind === "white") && g) {
                root.openGroup(g.id);
                root.openColors(g.id, kind === "white" ? "" : (g.lightIds[0] ?? ""));
                if (kind === "white") root.pageStack.currentItem.whiteMode = true;
            }
            if (root.screenshotPath.length > 0) grabTimer.start();
        }
    }
    Timer {
        id: grabTimer
        interval: 1200
        onTriggered: root.pageStack.grabToImage(r => {
            r.saveToFile(root.screenshotPath);
            Qt.quit();
        })
    }

    Component {
        id: setupPage
        SetupPage { app: root }
    }
    Component {
        id: homePage
        HomePage { app: root }
    }
    Component {
        id: groupPage
        GroupPage { app: root }
    }
    Component {
        id: colorPage
        ColorPage { app: root }
    }
}
