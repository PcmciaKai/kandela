import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import QtQuick.Shapes
import org.kde.kirigami as Kirigami

// A scene: a swirl of its colors with the name underneath. Click to recall.
QQC2.AbstractButton {
    id: tile

    required property var scene
    readonly property var colors: scene.colors.length > 0 ? scene.colors : ["#ffe0b0"]
    function colorAt(i) { return colors[i % colors.length]; }

    hoverEnabled: true
    implicitWidth: Kirigami.Units.gridUnit * 5.5
    implicitHeight: contentItem.implicitHeight
    Accessible.name: scene.name
    Accessible.description: scene.active ? qsTr("Active") : ""

    contentItem: ColumnLayout {
        spacing: Kirigami.Units.smallSpacing

        Item {
            Layout.alignment: Qt.AlignHCenter
            implicitWidth: Kirigami.Units.gridUnit * 4
            implicitHeight: implicitWidth

            // Selection ring.
            Rectangle {
                anchors.fill: parent
                radius: width / 2
                color: "transparent"
                border.width: 2
                border.color: tile.scene.active ? Kirigami.Theme.highlightColor
                            : tile.visualFocus ? Kirigami.Theme.focusColor
                            : tile.hovered ? Kirigami.Theme.hoverColor : "transparent"
            }

            Shape {
                id: swirl
                anchors.fill: parent
                anchors.margins: 5
                preferredRendererType: Shape.CurveRenderer
                scale: tile.pressed ? 0.94 : 1
                Behavior on scale { NumberAnimation { duration: Kirigami.Units.shortDuration } }

                ShapePath {
                    strokeWidth: 0
                    strokeColor: "transparent"
                    fillGradient: ConicalGradient {
                        centerX: swirl.width / 2
                        centerY: swirl.height / 2
                        angle: 90
                        GradientStop { position: 0.0; color: tile.colorAt(0) }
                        GradientStop { position: 0.33; color: tile.colorAt(1) }
                        GradientStop { position: 0.66; color: tile.colorAt(2) }
                        GradientStop { position: 1.0; color: tile.colorAt(0) }
                    }
                    PathAngleArc {
                        centerX: swirl.width / 2
                        centerY: swirl.height / 2
                        radiusX: swirl.width / 2
                        radiusY: swirl.height / 2
                        startAngle: 0
                        sweepAngle: 360
                    }
                }
            }

            // A soft highlight so the swirl reads as a glowing disc.
            Rectangle {
                anchors.fill: swirl
                radius: width / 2
                scale: swirl.scale
                gradient: Gradient {
                    GradientStop { position: 0.0; color: Qt.rgba(1, 1, 1, 0.35) }
                    GradientStop { position: 0.5; color: Qt.rgba(1, 1, 1, 0) }
                    GradientStop { position: 1.0; color: Qt.rgba(0, 0, 0, 0.12) }
                }
            }
        }

        QQC2.Label {
            Layout.fillWidth: true
            horizontalAlignment: Text.AlignHCenter
            text: tile.scene.name
            wrapMode: Text.Wrap
            maximumLineCount: 2
            elide: Text.ElideRight
            font.weight: tile.scene.active ? Font.DemiBold : Font.Normal
        }
    }
}
