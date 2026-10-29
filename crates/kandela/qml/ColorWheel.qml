import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Shapes
import org.kde.kirigami as Kirigami

// A hue/saturation wheel with one draggable marker per light. Angle is hue
// (red at 3 o'clock, counter-clockwise), distance from the center is
// saturation.
Item {
    id: wheel

    // [{ id, name, on, color, hue, saturation }]; dragging an off light turns it on.
    property var markers: []
    property string selectedId
    // Called with (lightId, hue, saturation) while a marker is dragged.
    signal colorPicked(string lightId, real hue, real saturation)
    // Called with (hue, saturation) when empty space is clicked and no light is selected.
    signal backgroundPicked(real hue, real saturation)
    signal markerSelected(string lightId)

    readonly property real radius: Math.min(width, height) / 2 - markerSize / 4
    readonly property real markerSize: Kirigami.Units.gridUnit * 2.2
    readonly property point center: Qt.point(width / 2, height / 2)

    // While dragging, the marker follows the pointer rather than the bridge.
    property string draggingId
    property real dragHue
    property real dragSaturation

    function positionFor(hue, saturation) {
        const a = hue * Math.PI / 180;
        const r = Math.min(1, saturation) * radius;
        return Qt.point(center.x + r * Math.cos(a), center.y - r * Math.sin(a));
    }

    function pickAt(x, y) {
        const dx = x - center.x, dy = center.y - y;
        let hue = Math.atan2(dy, dx) * 180 / Math.PI;
        if (hue < 0) hue += 360;
        return { hue: hue, saturation: Math.min(1, Math.sqrt(dx * dx + dy * dy) / radius) };
    }

    function markerAt(x, y) {
        let best = "", bestDist = markerSize * 0.75;
        markers.forEach(m => {
            const p = positionFor(m.hue, m.saturation);
            const d = Math.hypot(p.x - x, p.y - y);
            if (d < bestDist) { best = m.id; bestDist = d; }
        });
        return best;
    }

    Shape {
        id: disc
        x: wheel.center.x - wheel.radius
        y: wheel.center.y - wheel.radius
        width: wheel.radius * 2
        height: width
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeWidth: 0
            strokeColor: "transparent"
            fillGradient: ConicalGradient {
                centerX: disc.width / 2
                centerY: disc.height / 2
                angle: 0
                GradientStop { position: 0 / 6; color: "#ff0000" }
                GradientStop { position: 1 / 6; color: "#ffff00" }
                GradientStop { position: 2 / 6; color: "#00ff00" }
                GradientStop { position: 3 / 6; color: "#00ffff" }
                GradientStop { position: 4 / 6; color: "#0000ff" }
                GradientStop { position: 5 / 6; color: "#ff00ff" }
                GradientStop { position: 6 / 6; color: "#ff0000" }
            }
            PathAngleArc {
                centerX: disc.width / 2; centerY: disc.height / 2
                radiusX: disc.width / 2; radiusY: disc.height / 2
                sweepAngle: 360
            }
        }
        // Fading to white towards the center gives exactly HSV saturation.
        ShapePath {
            strokeWidth: 0
            strokeColor: "transparent"
            fillGradient: RadialGradient {
                centerX: disc.width / 2; centerY: disc.height / 2
                centerRadius: disc.width / 2
                focalX: centerX; focalY: centerY
                GradientStop { position: 0; color: "#ffffffff" }
                GradientStop { position: 1; color: "#00ffffff" }
            }
            PathAngleArc {
                centerX: disc.width / 2; centerY: disc.height / 2
                radiusX: disc.width / 2; radiusY: disc.height / 2
                sweepAngle: 360
            }
        }
    }

    MouseArea {
        anchors.fill: parent
        preventStealing: true
        onPressed: mouse => {
            const hit = wheel.markerAt(mouse.x, mouse.y);
            const id = hit || wheel.selectedId;
            const c = wheel.pickAt(mouse.x, mouse.y);
            if (hit) wheel.markerSelected(hit);
            if (!id) {
                wheel.backgroundPicked(c.hue, c.saturation);
                return;
            }
            wheel.draggingId = id;
            if (!hit) {
                wheel.dragHue = c.hue;
                wheel.dragSaturation = c.saturation;
                wheel.colorPicked(id, c.hue, c.saturation);
            } else {
                const m = wheel.markers.find(m => m.id === hit);
                wheel.dragHue = m.hue;
                wheel.dragSaturation = m.saturation;
            }
        }
        onPositionChanged: mouse => {
            if (!wheel.draggingId) return;
            const c = wheel.pickAt(mouse.x, mouse.y);
            wheel.dragHue = c.hue;
            wheel.dragSaturation = c.saturation;
            wheel.colorPicked(wheel.draggingId, c.hue, c.saturation);
        }
        onReleased: wheel.draggingId = ""
        onCanceled: wheel.draggingId = ""
    }

    Repeater {
        model: wheel.markers
        delegate: Kirigami.ShadowedRectangle {
            id: marker
            required property var modelData
            readonly property bool dragging: wheel.draggingId === modelData.id
            readonly property bool selected: wheel.selectedId === modelData.id
            readonly property point pos: dragging
                ? wheel.positionFor(wheel.dragHue, wheel.dragSaturation)
                : wheel.positionFor(modelData.hue, modelData.saturation)

            width: wheel.markerSize * (selected ? 1.15 : 1)
            height: width
            x: pos.x - width / 2
            y: pos.y - height / 2
            z: selected || dragging ? 2 : 1
            radius: width / 2
            color: dragging ? Qt.hsva(wheel.dragHue / 360, wheel.dragSaturation, 1, 1)
                 : modelData.on ? modelData.color : "#7f8c8d"
            opacity: modelData.on || dragging ? 1 : 0.7
            border.width: selected ? 3 : 2
            border.color: selected ? Kirigami.Theme.highlightColor : "white"
            shadow.size: Kirigami.Units.largeSpacing
            shadow.color: Qt.rgba(0, 0, 0, 0.35)
            shadow.yOffset: 1

            Behavior on width { NumberAnimation { duration: Kirigami.Units.shortDuration } }

            QQC2.Label {
                anchors.centerIn: parent
                text: marker.modelData.name.split(/\s+/).map(w => w[0] ?? "").join("").slice(0, 2).toUpperCase()
                color: "#202326"
                font.weight: Font.DemiBold
                font.pixelSize: parent.width * 0.32
            }

            QQC2.ToolTip.visible: hover.hovered
            QQC2.ToolTip.text: modelData.name
            HoverHandler { id: hover }
        }
    }
}
