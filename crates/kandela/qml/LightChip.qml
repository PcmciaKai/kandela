import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

// A pill-shaped toggle for choosing which light the color controls act on.
QQC2.AbstractButton {
    id: chip

    property color dotColor: "transparent"
    property bool showDot: true

    hoverEnabled: true
    padding: Kirigami.Units.smallSpacing
    leftPadding: Kirigami.Units.largeSpacing
    rightPadding: Kirigami.Units.largeSpacing
    implicitWidth: implicitContentWidth + leftPadding + rightPadding
    implicitHeight: implicitContentHeight + topPadding + bottomPadding

    background: Rectangle {
        radius: height / 2
        color: chip.checked ? Qt.alpha(Kirigami.Theme.highlightColor, 0.25)
             : chip.hovered ? Qt.alpha(Kirigami.Theme.textColor, 0.08) : "transparent"
        border.width: 1
        border.color: chip.checked || chip.visualFocus ? Kirigami.Theme.highlightColor
                    : Qt.alpha(Kirigami.Theme.textColor, 0.2)
    }

    contentItem: RowLayout {
        spacing: Kirigami.Units.smallSpacing
        Rectangle {
            visible: chip.showDot
            implicitWidth: Kirigami.Units.iconSizes.small * 0.75
            implicitHeight: implicitWidth
            radius: width / 2
            color: chip.dotColor
            border.width: 1
            border.color: Qt.alpha(Kirigami.Theme.textColor, 0.3)
        }
        QQC2.Label { text: chip.text }
    }
}
