import QtQuick
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami

// A warm-to-cool white bar with one draggable marker per light.
Item {
    id: bar

    required property var app
    // [{ id, name, on, mirek }]; dragging an off light turns it on.
    property var markers: []
    property string selectedId
    signal mirekPicked(string lightId, real mirek)
    signal backgroundPicked(real mirek)
    signal markerSelected(string lightId)

    readonly property real warm: 500
    readonly property real cool: 153
    readonly property real markerSize: Kirigami.Units.gridUnit * 2.2
    readonly property real trackLeft: markerSize / 2
    readonly property real trackWidth: width - markerSize

    property string draggingId
    property real dragMirek

    implicitHeight: markerSize * 2.5

    function xFor(mirek) {
        return trackLeft + (warm - mirek) / (warm - cool) * trackWidth;
    }
    function mirekAt(x) {
        const t = Math.max(0, Math.min(1, (x - trackLeft) / trackWidth));
        return warm - t * (warm - cool);
    }
    function markerAt(x) {
        let best = "", bestDist = markerSize * 0.75;
        markers.forEach(m => {
            const d = Math.abs(xFor(m.mirek) - x);
            if (d < bestDist) { best = m.id; bestDist = d; }
        });
        return best;
    }

    Rectangle {
        id: track
        x: bar.trackLeft - height / 2
        width: bar.trackWidth + height
        height: bar.markerSize * 1.4
        anchors.verticalCenter: parent.verticalCenter
        radius: height / 2
        border.width: 1
        border.color: Qt.rgba(0, 0, 0, 0.1)
        gradient: Gradient {
            orientation: Gradient.Horizontal
            GradientStop { position: 0.0; color: bar.app.hue.mirekColor(500) }
            GradientStop { position: 0.3; color: bar.app.hue.mirekColor(370) }
            GradientStop { position: 0.6; color: bar.app.hue.mirekColor(260) }
            GradientStop { position: 1.0; color: bar.app.hue.mirekColor(153) }
        }
    }

    MouseArea {
        anchors.fill: parent
        preventStealing: true
        onPressed: mouse => {
            const hit = bar.markerAt(mouse.x);
            const id = hit || bar.selectedId;
            if (hit) bar.markerSelected(hit);
            if (!id) {
                bar.backgroundPicked(bar.mirekAt(mouse.x));
                return;
            }
            bar.draggingId = id;
            if (hit) {
                bar.dragMirek = bar.markers.find(m => m.id === hit).mirek;
            } else {
                bar.dragMirek = bar.mirekAt(mouse.x);
                bar.mirekPicked(id, bar.dragMirek);
            }
        }
        onPositionChanged: mouse => {
            if (!bar.draggingId) return;
            bar.dragMirek = bar.mirekAt(mouse.x);
            bar.mirekPicked(bar.draggingId, bar.dragMirek);
        }
        onReleased: bar.draggingId = ""
        onCanceled: bar.draggingId = ""
    }

    Repeater {
        model: bar.markers
        delegate: Kirigami.ShadowedRectangle {
            id: marker
            required property var modelData
            required property int index
            readonly property bool dragging: bar.draggingId === modelData.id
            readonly property bool selected: bar.selectedId === modelData.id
            readonly property real mirek: dragging ? bar.dragMirek : modelData.mirek

            width: bar.markerSize * (selected ? 1.15 : 1)
            height: width
            x: bar.xFor(mirek) - width / 2
            anchors.verticalCenter: parent.verticalCenter
            z: selected || dragging ? 2 : 1
            radius: width / 2
            color: modelData.on || dragging ? bar.app.hue.mirekColor(mirek) : "#7f8c8d"
            opacity: modelData.on || dragging ? 1 : 0.7
            border.width: selected ? 3 : 2
            border.color: selected ? Kirigami.Theme.highlightColor : "white"
            shadow.size: Kirigami.Units.largeSpacing
            shadow.color: Qt.rgba(0, 0, 0, 0.35)
            shadow.yOffset: 1

            QQC2.Label {
                anchors.centerIn: parent
                text: marker.modelData.name.split(/\s+/).map(w => w[0] ?? "").join("").slice(0, 2).toUpperCase()
                color: "#202326"
                font.weight: Font.DemiBold
                font.pixelSize: parent.width * 0.32
            }

            QQC2.ToolTip.visible: hover.hovered
            QQC2.ToolTip.text: modelData.name + " · " + Math.round(1000000 / marker.mirek) + " K"
            HoverHandler { id: hover }
        }
    }
}
